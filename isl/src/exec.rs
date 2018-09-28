use vm;

use data::Literal;
use errors::*;
use tokio::prelude::future::{loop_fn, ok, Future, Loop};
use tokio::runtime::Runtime;

pub struct Exec {
    runtime: Runtime,
}

impl Exec {
    pub fn new() -> Exec {
        Exec {
            runtime: Runtime::new().unwrap(),
        }
    }

    pub fn sched(&mut self, vm: vm::VM, code: vm::bytecode::Bytecode) -> Result<(vm::VM, Literal)> {
        let f = loop_fn(vm, move |mut vm| {
            vm.import_jump(&code);

            ok(vm).and_then(|mut vm| {
                let res = vm.step_until_cost(10000);

                match res {
                    Ok(Some(ret)) => Ok(Loop::Break((vm, ret))),
                    Ok(None) => Ok(Loop::Continue(vm)),
                    Err(e) => Err(e),
                }
            })
        });

        /*let f = f.then(|res| {
            println!("{:?}", res);
            ok::<(), ()>(())
        });*/

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
                    Op::Lit(1.into()),
                    Op::Lit("print".into()),
                    Op::Load,
                    Op::CallArity(1),
                    Op::Return,
                ]]),
            )
            .unwrap();

        assert_eq!(lit, 1.into());
    }
}
