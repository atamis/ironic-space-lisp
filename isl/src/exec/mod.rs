//! Parallel processing environment for ISL VMs.
//!
//! Warning: this code calls `unwrap` constantly, and probably panics all the time.
use crate::data;
use crate::data::Literal;
use crate::errors::*;
use crate::exec::router::router;
use crate::exec::router::RouterChan;
use crate::vm;
use async_trait::async_trait;
use futures::channel::mpsc;
use futures::future::{self, Future, FutureExt};
use futures::stream::StreamExt;
use std::fmt;
use std::pin::Pin;
use tokio::runtime::Runtime;

pub mod router;

pub use crate::exec::router::RouterMessage;

/// A trait for interfacing between a [`vm::VM`] and its execution environment.
#[async_trait]
pub trait ExecHandle: Send + Sync + fmt::Debug {
    /// Return the `Pid`, or unique identifier of the exec handle.
    fn get_pid(&mut self) -> data::Pid;
    /// Send a message to a particular `Pid`.
    fn send(&mut self, pid: data::Pid, msg: Literal) -> Result<()>;
    /// Spawn a new `VM`, consuming the `VM` and returning its `Pid`.
    fn spawn(&mut self, vm: vm::VM) -> Result<data::Pid>;
    /// Watch this PID
    fn watch(&mut self, watched: data::Pid) -> Result<()>;
    /// Asynchronously receive a Literal from your inbox.
    async fn receive(&mut self) -> Option<Literal>;
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
            rx,
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
        self.rx.next().await
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

    fn watch(&mut self, watched: data::Pid) -> Result<()> {
        Ok(self
            .router
            .try_send(RouterMessage::Watch(self.pid, watched))
            .context("Error sending on a router channel")?)
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
        if let Err(e) = self.router.try_send(RouterMessage::Close(self.pid)) {
            eprintln!("Error encountered while closing RouterHandle: {:?}", e);
        }
    }
}

fn exec_future(
    mut vm: vm::VM,
    router: &RouterChan,
) -> (
    data::Pid,
    Pin<Box<impl Future<Output = (vm::VM, Result<data::Literal>)>>>,
) {
    use crate::vm::VMState;

    // Whether or not the VM already has a proc
    // If it does, we don't want to replace it, or remove it later.
    let mut has_proc = false;

    let pid = if vm.proc.is_none() {
        let handle = RouterHandle::new(router.clone());

        let pid = handle.pid;

        vm.proc = Some(Box::new(handle));
        pid
    } else {
        has_proc = true;
        vm.proc.as_mut().unwrap().get_pid()
    };

    let f2 = async move || loop {
        vm.state = VMState::RunningUntil(100);

        if let Err(e) = vm.state_step() {
            eprintln!("Encountered error while running vm: {:?} ", e);
            return (vm, Err(e));
        };

        if let VMState::Done(_) = vm.state {
            let l = vm.state.get_ret().unwrap();
            if !has_proc {
                vm.proc = None;
            }
            return (vm, Ok(l));
        }

        if let VMState::Waiting = vm.state {
            let opt_lit = vm
                .proc
                .as_mut()
                .map(move |proc| proc.receive())
                .unwrap()
                .await
                .unwrap();
            vm.answer_waiting(opt_lit).unwrap()
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
    ) -> (vm::VM, Result<Literal>) {
        vm.import_jump(code);
        let (_, f) = exec_future(vm, &self.router_chan);

        self.runtime.block_on(f)
    }

    /// Wait for all futures to resolve.
    pub fn wait(mut self) {
        if let Err(e) = self.router_chan.try_send(RouterMessage::Quit) {
            eprintln!("Encountered error shutting down router: {:?}", e);
        }
        //self.runtime.shutdown_on_idle();
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

        let mut vm_handle = exec.get_handle();

        let vm_pid = vm_handle.get_pid();

        let mut vm = empty_vm();

        vm.proc = Some(Box::new(vm_handle));

        let mut my_handle = exec.get_handle();

        my_handle.send(vm_pid, "dummy-message".into()).unwrap();

        let (_, lit) = exec.sched(
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

        let (_, lit) = exec.sched(
            vm,
            &vm::bytecode::Bytecode::new(vec![vec![
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

    #[test]
    fn test_watches() {
        let mut runtime = Runtime::new().unwrap();
        let router = router(&mut runtime);

        let mut handle1 = RouterHandle::new(router.clone());
        let mut handle2 = RouterHandle::new(router.clone());

        let watched_pid = handle2.get_pid();

        handle1.watch(watched_pid).unwrap();

        drop(handle2);

        let msg = executor::block_on(handle1.receive()).unwrap();

        assert_eq!(
            msg,
            list_lit![data::Literal::Keyword("exit".into()), watched_pid]
        );
    }
}
