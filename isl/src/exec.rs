use data;
use data::Literal;
use errors::*;
use std::collections::HashMap;
use tokio::prelude::future::{loop_fn, ok, Future, Loop};
use tokio::prelude::stream::Stream;
use tokio::runtime::Runtime;
use tokio_channel::mpsc;
use vm;

pub type RouterChan = mpsc::Sender<RouterMessage>;

#[derive(Debug)]
pub struct ProcInfo {
    pub pid: data::Pid,
    pub chan: RouterChan,
}

type RouterState = HashMap<data::Pid, mpsc::Sender<Literal>>;

#[derive(Debug)]
pub enum RouterMessage {
    Close(data::Pid),
    Register(data::Pid, mpsc::Sender<Literal>),
    Send(data::Pid, Literal),
}

pub struct RouterHandle {
    pid: data::Pid,
    rx: Option<mpsc::Receiver<Literal>>,
    router: RouterChan,
}

impl RouterHandle {
    pub fn new(mut chan: RouterChan) -> RouterHandle {
        let pid = data::Pid::gen();
        let (tx, rx) = mpsc::channel::<Literal>(10);
        chan.try_send(RouterMessage::Register(pid, tx)).unwrap();

        RouterHandle {
            pid,
            rx: Some(rx),
            router: chan,
        }
    }

    pub fn receive(mut self) -> impl Future<Item = (Literal, RouterHandle), Error = ()> {
        use std::mem;
        let rx = mem::replace(&mut self.rx, None).unwrap();

        rx.into_future().then(move |res| {
            let (msg, rx) = res.unwrap();
            mem::replace(&mut self.rx, Some(rx));
            ok::<(Literal, RouterHandle), ()>((msg.unwrap(), self))
        })
    }

    pub fn send(&mut self, pid: data::Pid, msg: Literal) {
        self.router.try_send(RouterMessage::Send(pid, msg)).unwrap()
    }

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
            ok(state)
        })
        .then(|x| {
            println!("Router exited: {:?}", x);
            ok::<(), ()>(())
        });

    runtime.spawn(f);

    tx
}

pub struct Exec {
    runtime: Runtime,
    router_chan: RouterChan,
}

impl Exec {
    pub fn new() -> Exec {
        let mut runtime = Runtime::new().unwrap();

        let tx = router(&mut runtime);

        Exec {
            runtime,
            router_chan: tx,
        }
    }

    pub fn get_handle(&self) -> RouterHandle {
        RouterHandle::new(self.router_chan.clone())
    }

    /// Schedule a VM for execution on some bytecode.
    pub fn sched(
        &mut self,
        mut vm: vm::VM,
        code: &vm::bytecode::Bytecode,
    ) -> Result<(vm::VM, Literal)> {
        use vm::VMState;

        let handle = RouterHandle::new(self.router_chan.clone());

        vm.proc = Some(handle.get_procinfo());

        self.router_chan
            .try_send(RouterMessage::Send(handle.pid, "dummy-message".into()))
            .unwrap();

        vm.import_jump(&code);

        let f = loop_fn((vm, handle), move |(vm, handle)| {
            ok((vm, handle)).and_then(
                |(mut vm, handle)| -> Box<
                    Future<
                            Item = Loop<(vm::VM, Literal), (vm::VM, RouterHandle)>,
                            Error = failure::Error,
                        > + Send,
                > {
                    vm.state = VMState::RunningUntil(100);
                    vm.state_step().unwrap();

                    if let VMState::Done(_) = vm.state {
                        let l = { vm.state.get_ret().unwrap() };
                        vm.proc = None;
                        return Box::new(ok(Loop::Break((vm, l))));
                    }

                    if let VMState::Stopped = vm.state {
                        return Box::new(ok(Loop::Continue((vm, handle))));
                    }

                    if let VMState::Waiting = vm.state {
                        return Box::new(handle.receive().then(|res| {
                            let (opt_lit, handle) = res.unwrap();
                            vm.answer_waiting(opt_lit).unwrap();
                            Ok(Loop::Continue((vm, handle)))
                        }));
                    }

                    panic!("VM state not done, stopped, or waiting");
                },
            )
        });

        self.runtime.block_on(f)
    }

    pub fn run(&mut self) {}

    pub fn wait(self) {
        self.runtime.shutdown_on_idle().wait().unwrap();
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
    use vm::op::Op;

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

        let mut vm = empty_vm();

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

        let mut vm = empty_vm();

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
        let (msg, mut handle2) = handle2.receive().wait().unwrap();
        assert_eq!(msg, "test-message".into());

        handle2.send(handle1.pid, "test-message2".into());
        let (msg, _) = handle1.receive().wait().unwrap();
        assert_eq!(msg, "test-message2".into());
    }
}
