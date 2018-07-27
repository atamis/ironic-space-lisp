extern crate ironic_space_lisp;


use std::rc::Rc;

use ironic_space_lisp::vm;
use ironic_space_lisp::vm::Op;
use ironic_space_lisp::vm::data;

fn debug_single_step(vm: &mut vm::VM) {
    vm.single_step();

    println!("{:?}", vm);
}

fn main() {
    let inst = vec![Op::Lit(data::Literal::Number(4)),
                    Op::Lit(data::Literal::Number(4)),
                    Op::Lit(data::Literal::Builtin(Rc::new(vm::AdditionFunction))),
                    Op::ApplyFunction,
                    Op::Lit(data::Literal::Lambda(Rc::new(vm::AddOneFunction))),
                    Op::ApplyFunction,
                    Op::ReturnOp,
    ];

    let mut vm = vm::VM::new(inst);

    vm.step_until_value(true);

    println!("{:?}", vm);

}
