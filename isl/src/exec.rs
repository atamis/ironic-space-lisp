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
use async_trait::async_trait;

/// A channel to the message router.
pub type RouterChan = mpsc::Sender<RouterMessage>;

/// A trait for interfacing between a [`vm::VM`] and its execution environment.
#[async_trait]
pub trait ExecHandle: Send + Sync + fmt::Debug {
    /// Return the `Pid`, or unique identifier of the exec handle.
    fn get_pid(&mut self) -> data::Pid;
    /// Send a message to a particular `Pid`.
    fn send(&mut self, pid: data::Pid, msg: Literal) -> Result<()>;
    /// Spawn a new `VM`, consuming the `VM` and returning its `Pid`.
    fn spawn(&mut self, vm: vm::VM) -> Result<data::Pid>;
    /// Asynchronously receive a Literal from your inbox.
    async fn receive(&mut self) -> Option<Literal>;
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
    /// Safely close the router once all other handlers are dropped..
    Quit,
}

/// Represents a handle on a Router.
///
/// Automatically manages registration and deregistration.
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
}

#[async_trait]
impl ExecHandle for RouterHandle {
    fn get_pid(&mut self) -> data::Pid {
        self.pid
    }

    /// Asynchronously receive a Literal from this channel.
    async fn receive(&mut self) -> Option<Literal> {
        self.rx
            .next()
            .await
    }

    /// Send a message through  to a pid.
    fn send(&mut self, pid: data::Pid, msg: Literal) -> Result<()> {
        Ok(self
            .router
            .try_send(RouterMessage::Send(pid, msg))
            .context("Error sending on router channel")?)
    }

    fn spawn(&mut self, vm: vm::VM) -> Result<data::Pid> {
        let (pid, f) = exec_future(vm, &self.router);
        let f = f.then(|_| future::ready(()));

        tokio::spawn(f);

        Ok(pid)
    }
}

impl Clone for RouterHandle {
    fn clone(&self) -> Self {
        RouterHandle::new(self.router.clone())
    }
}

impl fmt::Debug for RouterHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Derive implementation includes all fields, most of which
        // aren't relevant.
       write!(f, "RouterHandle({:?})", self.pid)
    }
}


impl Drop for RouterHandle {
    fn drop(&mut self) {
        if let Err(e) = self.router .try_send(RouterMessage::Close(self.pid)) {
            eprintln!("Error encountered while closing RouterHandle: {:?}", e);
        }
    }
}

/// Spawn a router on the runtime.
///
/// Routers respond to router messages sent on the sender channel this function returns.
pub fn router(runtime: &mut Runtime) -> mpsc::Sender<RouterMessage> {
    let (tx, rx) = mpsc::channel::<RouterMessage>(10);

    let f = async move || {
        let mut rx = rx;
        let mut state = RouterState::new();
        let mut quitting = false;

        loop {
            if quitting && state.is_empty() {
                    break;
            }

            let msg = rx.next().await;

            println!("Recieved message {:?}", msg);

            match msg {
                None => break,
                Some(RouterMessage::Close(p)) => {
                    state.remove(&p);
                }
                Some(RouterMessage::Register(p, tx)) => {
                    state.insert(p, tx);
                }
                Some(RouterMessage::Send(p, l)) => {
                    //state.get_mut(&p).unwrap(),
                    if let Some(chan) = state.get_mut(&p) {
                        if let Err(e) = chan.try_send(l) {
                            eprintln!("Attempted to send on closed channel {:?}, but encountered error: {:?}", p, e);
                            state.remove(&p);
                        }
                    } else {
                        eprintln!("Attempted to send to non-existant pid {:?}: {:?}", p, l)
                    }
                },
                Some(RouterMessage::Quit) => quitting = true,
            };
        }

        println!("Router finished (quitting: {:?}): {:?}", quitting,  state);

        ()
        };

    runtime.spawn(f());

    tx
}

fn exec_future(
    mut vm: vm::VM,
    router: &RouterChan,
) -> (
    data::Pid,
    Pin<Box<impl Future<Output = (vm::VM, Result<data::Literal>)>>>,
) {
    use crate::vm::VMState;

    let mut handle = RouterHandle::new(router.clone());

    let pid = handle.pid;

    handle.send(handle.pid, "dummy-message".into()).unwrap();

    vm.proc = Some(Box::new(handle));

    let f2 = async move || {
        loop {
            vm.state = VMState::RunningUntil(100);

            if let Err(e) = vm.state_step() {
                return (vm, Err(e))
            };

            if let VMState::Done(_) = vm.state {
                let l = { vm.state.get_ret().unwrap() };
                vm.proc = None;
                return (vm, Ok(l));
            }

            if let VMState::Waiting = vm.state {
                let opt_lit = vm.proc.as_mut().map(move |proc| proc.receive()).unwrap().await.unwrap();
                vm.answer_waiting(opt_lit).unwrap()
            }
           
        }
    };

    (pid, Box::pin(f2()))
}

/// Holds handles to its Runtime and router.
pub struct Exec {
    /// The Tokio Runtime this Exec uses. All VMs and the router
    /// get launched on this runtime.
    pub runtime: Runtime,
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
    ) -> ( vm::VM, Result<Literal> )
    {
        vm.import_jump(code);
        let (_, f) = exec_future(vm, &self.router_chan);

        self.runtime.block_on(f)
    }

    /// Wait for all futures to resolve.
    pub fn wait(mut self) {
        self.router_chan.try_send(RouterMessage::Quit);
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
            );

        let lit = lit.unwrap();

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
            );

        assert_eq!(lit.unwrap(), "from-myself".into());
    }

    #[test]
    fn test_handle() {
        let mut runtime = Runtime::new().unwrap();
        let router = router(&mut runtime);

        let mut handle1 = RouterHandle::new(router.clone());
        let mut handle2 = RouterHandle::new(router.clone());

        handle1.send(handle2.pid, "test-message".into()).unwrap();
        let msg = executor::block_on(handle2.receive()).unwrap();
        assert_eq!(msg, "test-message".into());

        handle2.send(handle1.pid, "test-message2".into()).unwrap();
        let msg = executor::block_on(handle1.receive()).unwrap();
        assert_eq!(msg, "test-message2".into());
    }
}
