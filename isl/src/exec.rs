//! Parallel processing environment for ISL VMs.
//!
//! Warning: this code calls `unwrap` constantly, and probably panics all the time.
use crate::data;
use crate::data::Literal;
use crate::errors::*;
use crate::vm;
use futures::channel::mpsc;
use futures::future::{Future, FutureExt, self};
use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use tokio::runtime::Runtime;
use futures::stream::StreamExt;

/// A channel to the message router.
pub type RouterChan = mpsc::Sender<RouterMessage>;

/// Inserted into VMs to allow them to send messages to the router, and know their `Pid`.
#[derive(Debug)]
pub struct ProcInfo {
    /// The [`Pid`](data::Pid), or unique identifier, for this handle.
    pub pid: data::Pid,
    /// A channel back to the central router for this executor.
    pub chan: RouterChan,
}

/// A trait for interfacing between a [`vm::VM`] and its execution environment.
pub trait ExecHandle: Send + Sync + fmt::Debug {
    /// Return the `Pid`, or unique identifier of the exec handle.
    fn get_pid(&mut self) -> data::Pid;
    /// Send a message to a particular `Pid`.
    fn send(&mut self, recv: data::Pid, msg: Literal) -> Result<()>;
    /// Spawn a new `VM`, consuming the `VM` and returning its `Pid`.
    fn spawn(&mut self, vm: vm::VM) -> Result<data::Pid>;
}

impl ExecHandle for ProcInfo {
    fn get_pid(&mut self) -> data::Pid {
        self.pid
    }

    fn send(&mut self, recv: data::Pid, msg: Literal) -> Result<()> {
        Ok(self
            .chan
            .try_send(RouterMessage::Send(recv, msg))
            .context("Error sending on router channel")?)
    }

    fn spawn(&mut self, vm: vm::VM) -> Result<data::Pid> {
        let (pid, f) = exec_future(vm, &self.chan);
        let f = f.then(|_| future::ready(()));

        tokio::spawn(f);

        Ok(pid)
    }
}

type RouterState = HashMap<data::Pid, mpsc::Sender<Literal>>;

/// Messages you can send to the router.
#[derive(Debug)]
pub enum RouterMessage {
    /// Deregister a Pid.
    Close(data::Pid),
    /// Register a Pid with a Sender channel.
    Register(data::Pid, mpsc::Sender<Literal>),
    /// Send some data to the channel associated with a Pid.
    Send(data::Pid, Literal),
}

/// Represents a handle on a Router.
///
/// Automatically manages registration and deregistration. Can't implement clone
/// because the channel receiver can't be cloned.
pub struct RouterHandle {
    pid: data::Pid,
    rx: mpsc::Receiver<Literal>,
    router: RouterChan,
}

impl RouterHandle {
    /// Register with a router, returning the handle.
    pub fn new(mut chan: RouterChan) -> RouterHandle {
        let pid = data::Pid::gen();
        let (tx, rx) = mpsc::channel::<Literal>(10);
        chan.try_send(RouterMessage::Register(pid, tx)).unwrap();

        RouterHandle {
            pid,
            rx: rx,
            router: chan,
        }
    }

    /// Asynchronously receive a Literal from this channel.
    pub async fn receive(&mut self) -> Literal {
        self.rx
            .next()
            .await
            .unwrap()
    }

    /// Send a message through  to a pid.
    pub fn send(&mut self, pid: data::Pid, msg: Literal) {
        self.router.try_send(RouterMessage::Send(pid, msg)).unwrap()
    }

    /// Returns a procinfo suitable for inserting into a VM associated with this handle.
    pub fn get_procinfo(&self) -> ProcInfo {
        ProcInfo {
            pid: self.pid,
            chan: self.router.clone(),
        }
    }
}

impl Drop for RouterHandle {
    fn drop(&mut self) {
        self.router
            .try_send(RouterMessage::Close(self.pid))
            .unwrap();
    }
}

/// Spawn a router on the runtime.
///
/// Routers respond to router messages sent on the sender channel this function returns.
pub fn router(runtime: &mut Runtime) -> mpsc::Sender<RouterMessage> {
    let (tx, rx) = mpsc::channel::<RouterMessage>(10);

    let f = rx
        .fold(RouterState::new(), |mut state, msg| {
            match msg {
                RouterMessage::Close(p) => {
                    state.remove(&p);
                }
                RouterMessage::Register(p, tx) => {
                    state.insert(p, tx);
                }
                RouterMessage::Send(p, l) => state.get_mut(&p).unwrap().try_send(l).unwrap(),
            };
            future::ready(state)
        })
        .then(|x| {
            println!("Router exited: {:?}", x);
            future::ready(())
        });


    runtime.spawn(f);

    tx
}

fn exec_future(
    mut vm: vm::VM,
    router: &RouterChan,
) -> (
    data::Pid,
    Pin<Box<impl Future<Output = Result<(vm::VM, data::Literal)>>>>,
) {
    use crate::vm::VMState;

    let mut handle = RouterHandle::new(router.clone());

    let proc = handle.get_procinfo();

    let pid = proc.pid;

    vm.proc = Some(Box::new(proc));

    handle
        .router
        .try_send(RouterMessage::Send(handle.pid, "dummy-message".into()))
        .unwrap();

    let f2 = async move || {
        loop {
            vm.state = VMState::RunningUntil(100);
            vm.state_step()?;

            if let VMState::Done(_) = vm.state {
                let l = { vm.state.get_ret().unwrap() };
                vm.proc = None;
                return Ok((vm, l));
            }

            if let VMState::Waiting = vm.state {
                let opt_lit = handle.receive().await;
                vm.answer_waiting(opt_lit).unwrap()
            }
        }
    };

    (pid, Box::pin(f2()))
}

/// Holds handles to its Runtime and router.
pub struct Exec {
    runtime: Runtime,
    router_chan: RouterChan,
}

impl Exec {
    /// Spawn and take ownership of a Runtime and router.
    pub fn new() -> Exec {
        let mut runtime = Runtime::new().unwrap();

        let tx = router(&mut runtime);

        Exec {
            runtime,
            router_chan: tx,
        }
    }

    /// Get a new router handle to this Exec's Router.
    pub fn get_handle(&self) -> RouterHandle {
        RouterHandle::new(self.router_chan.clone())
    }

    /// Schedule a VM for execution on some bytecode.
    pub fn sched(
        &mut self,
        mut vm: vm::VM,
        code: &vm::bytecode::Bytecode,
    ) -> Result<(vm::VM, Literal)>
    {
        vm.import_jump(code);
        let (_, f) = exec_future(vm, &self.router_chan);

        self.runtime.block_on(f)
    }

    /// Wait for all futures to resolve.
    pub fn wait(self) {
        self.runtime.shutdown_on_idle();
    }
}

impl Default for Exec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::op::Op;
    use futures::executor;

    fn empty_vm() -> vm::VM {
        let mut builder = vm::Builder::new();

        builder.default_libs().print_trace(false);

        let (res, vm) = builder.build_exec();
        res.unwrap();
        vm
    }

    #[test]
    fn test_exec() {
        let mut exec = Exec::new();

        let vm = empty_vm();

        let (_, lit) = exec
            .sched(
                vm,
                &vm::bytecode::Bytecode::new(vec![vec![
                    //Op::Lit(1.into()),
                    Op::Wait,
                    Op::Lit("print".into()),
                    Op::Load,
                    Op::CallArity(1),
                    Op::Return,
                ]]),
            )
            .unwrap();

        assert_eq!(lit, "dummy-message".into());
        println!("{:?}", lit);
    }

    #[test]
    fn test_pid_send() {
        let mut exec = Exec::new();

        let vm = empty_vm();

        let (_, lit) = exec
            .sched(
                vm,
                &vm::bytecode::Bytecode::new(vec![vec![
                    Op::Wait,
                    Op::Pop, // throw away dummy message
                    Op::Lit("from-myself".into()),
                    Op::Pid,
                    Op::Send,
                    Op::Wait,
                    Op::Return,
                ]]),
            )
            .unwrap();

        assert_eq!(lit, "from-myself".into());
    }

    #[test]
    fn test_handle() {
        let mut runtime = Runtime::new().unwrap();
        let router = router(&mut runtime);

        let mut handle1 = RouterHandle::new(router.clone());
        let mut handle2 = RouterHandle::new(router.clone());

        handle1.send(handle2.pid, "test-message".into());
        let msg = executor::block_on(handle2.receive());
        assert_eq!(msg, "test-message".into());

        handle2.send(handle1.pid, "test-message2".into());
        let msg = executor::block_on(handle1.receive());
        assert_eq!(msg, "test-message2".into());
    }
}
