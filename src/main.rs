extern crate ironic_space_lisp;

use std::rc::Rc;

use ironic_space_lisp::vm;
use ironic_space_lisp::vm::Op;
use ironic_space_lisp::data;
use ironic_space_lisp::lisp::*;

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

    vm.step_until_value(true).unwrap();

    println!("{:?}", vm);

    let prog = make_list(
        vec![
        Lisp::Op(SimOp::Add),
        Lisp::Num(1),
        make_list(
            vec![
            Lisp::Op(SimOp::Add),
            Lisp::Num(1),
            Lisp::Num(1),
                ]
        )
                ]
    );

    let mut evaler = Evaler::new(prog);
    //let mut evaler = Evaler::new(Lisp::Num(1));
    println!("{:?}", evaler);

    println!("{:?}", evaler.step_until_return());
}

fn single_step_debug(evaler: &mut Evaler) {
    evaler.single_step().unwrap();
    println!("{:?}", evaler);
}
