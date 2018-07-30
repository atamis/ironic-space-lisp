extern crate ironic_space_lisp;


use std::rc::Rc;

use ironic_space_lisp::vm;
use ironic_space_lisp::vm::Op;
use ironic_space_lisp::data;
use ironic_space_lisp::errors::*;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let inst = vec![Op::Lit(data::Literal::Number(4)),
                    Op::Lit(data::Literal::Number(4)),
                    Op::Lit(data::Literal::Builtin(Rc::new(vm::AdditionFunction))),
                    Op::ApplyFunction,
                    Op::Lit(data::Literal::Lambda(Rc::new(vm::AddOneFunction))),
                    Op::ApplyFunction,
                    Op::ReturnOp,
    ];

    let mut vm = vm::VM::new(inst);

    vm.step_until_value(true)?;

    println!("{:?}", vm);

    Ok(())
}
