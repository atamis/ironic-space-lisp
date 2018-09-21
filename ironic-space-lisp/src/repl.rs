use rustyline::error::ReadlineError;
use rustyline::Editor;

use ast::passes::function_lifter;
use ast::passes::internal_macro;
use ast::passes::unbound;
use compiler;
use data;
use errors::*;
use size::*;
use str_to_ast;
use vm;

pub fn repl() {
    let mut vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));

    let mut rl = Editor::<()>::new();

    loop {
        let readline = rl.readline(&format!("{:} {:?} >", vm.code.chunks.len(), vm.data_size()));

        let mut res = Err(err_msg("No relevant matches error"));

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_ref());
                res = eval(&mut vm, &line);
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
            }
        }

        if let Err(ref e) = res {
            vm.code.dissassemble();
            println!("{:?}", vm);
            println!("error: {}", e);

            for e in e.iter_causes() {
                println!("caused by: {}", e);
            }

            // The backtrace is not always generated. Try to run this example
            // with `RUST_BACKTRACE=1`.
            if let Some(backtrace) = Some(e.backtrace()) {
                println!("backtrace: {:?}", backtrace);
            }
        } else {
            println!("{:?}", res.unwrap());
            //println!("{:?}", vm);
            //vm.code.dissassemble();
        }
    }
}

pub fn eval(vm: &mut vm::VM, s: &str) -> Result<Option<data::Literal>> {
    let ast = str_to_ast(&s)?;

    let ast = internal_macro::pass(&ast)?;

    unbound::pass(&ast, vm.environment.peek()?).context("Unbound pass in repl")?;

    let last = function_lifter::lift_functions(&ast)?;

    let code = compiler::pack_compile_lifted(&last)?;

    vm.import_jump(&code);

    let val = vm.step_until_cost(10000)?;

    Ok(val)
}
