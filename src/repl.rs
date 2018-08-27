use std::io;
use std::io::prelude::*;

use vm;
use compiler;
use ast::passes::unbound;
use ast::passes::function_lifter;
use errors::*;
use str_to_ast;

pub fn repl() {
    let mut vm = vm::VM::new(vm::Bytecode::new(vec![]));
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let res = eval(&mut vm, &line);

        if let Err(ref e) = res {
            println!("error: {}", e);

            vm.code.dissassemble();
            println!("{:?}", vm);

            for e in e.iter_causes() {
                println!("caused by: {}", e);
            }

            // The backtrace is not always generated. Try to run this example
            // with `RUST_BACKTRACE=1`.
            if let Some(backtrace) = Some(e.backtrace()) {
                println!("backtrace: {:?}", backtrace);
            }

        }

    }
}

pub fn eval(vm: &mut vm::VM, s: &str) -> Result<()> {

    let ast = str_to_ast(&s)?;

    unbound::pass(&ast, vm.environment.peek()?).context("Unbound pass in repl")?;

    let last = function_lifter::lift_functions(&ast)?;

    let code = compiler::pack_compile_lifted(&last)?;

    vm.import_jump(&code);

    let val = vm.step_until_cost(10000)?;

    println!("{:?}", val);

    Ok(())
}
