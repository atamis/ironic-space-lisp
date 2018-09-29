use data;
use data::Literal;
use errors::*;
use std::collections::HashMap;
use tokio::prelude::future::{loop_fn, ok, Future, Loop};
use tokio::prelude::stream::Stream;
use tokio::runtime::Runtime;
use tokio_channel::mpsc;
use vm;

type RouterState = HashMap<data::Pid, mpsc::Sender<Literal>>;

enum RouterMessage {
    Close(data::Pid),
    Register(data::Pid, mpsc::Sender<Literal>),
    Send(data::Pid, Literal),
}

pub struct Exec {
    runtime: Runtime,
    router_chan: mpsc::Sender<RouterMessage>,
}

impl Exec {
    pub fn new() -> Exec {
        let mut runtime = Runtime::new().unwrap();

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

        Exec {
            runtime,
            router_chan: tx,
        }
    }

    /// Schedule a VM for execution on some bytecode.
    pub fn sched(
        &mut self,
        mut vm: vm::VM,
        code: vm::bytecode::Bytecode,
    ) -> Result<(vm::VM, Literal)> {
        use vm::VMState;

        let (mut tx, rx) = mpsc::channel::<Literal>(10);
        let pid = data::Pid::gen();

        self.router_chan
            .try_send(RouterMessage::Register(pid, tx))
            .unwrap();
        self.router_chan
            .try_send(RouterMessage::Send(pid, "dummy-message".into()))
            .unwrap();

        vm.import_jump(&code);

        let f = loop_fn((vm, rx), move |(vm, rx)| {
            ok((vm, rx)).and_then(
                |(mut vm, rx)| -> Box<
                    Future<
                            Item = Loop<(vm::VM, Literal), (vm::VM, mpsc::Receiver<Literal>)>,
                            Error = failure::Error,
                        > + Send,
                > {
                    vm.state = VMState::RunningUntil(100);
                    vm.state_step().unwrap();

                    if let VMState::Done(_) = vm.state {
                        let l = { vm.state.get_ret().unwrap() };
                        return Box::new(ok(Loop::Break((vm, l))));
                    }

                    if let VMState::Stopped = vm.state {
                        return Box::new(ok(Loop::Continue((vm, rx))));
                    }

                    if let VMState::Waiting = vm.state {
                        return Box::new(rx.into_future().then(|res| {
                            let (opt_lit, rx) = res.unwrap();
                            vm.answer_waiting(opt_lit.unwrap()).unwrap();
                            Ok(Loop::Continue((vm, rx)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use vm::op::Op;

    fn empty_vm() -> vm::VM {
        let mut builder = vm::Builder::new();

        builder.default_libs().print_trace(true);

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
                vm::bytecode::Bytecode::new(vec![vec![
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
}
