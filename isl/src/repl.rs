//! Run an interactive REPL on a [`vm::VM`].

use rustyline::error::ReadlineError;
use rustyline::Editor;

use crate::ast::passes::function_lifter;
use crate::ast::passes::internal_macro;
use crate::ast::passes::local;
use crate::ast::passes::unbound;
use crate::compiler;
use crate::data;
use crate::errors::*;
use crate::size::*;
use crate::str_to_ast;
use crate::vm;



/// Run a REPL executing on a [`vm::VM`].
pub fn repl() {
    let mut vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));

    let mut rl = Editor::<()>::new();

    loop {
        let readline = rl.readline(&format!("{:} {:?} >", vm.code.chunks.len(), vm.data_size()));

        let mut res = Err(err_msg("No relevant matches error"));

        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
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

/// Parse a string and evaluate it on the VM with a limited resource pool of 10000 cost.
pub fn eval(vm: &mut vm::VM, s: &str) -> Result<Option<data::Literal>> {
    let ast = str_to_ast(&s)?;

    let ast = internal_macro::pass(&ast)?;

    unbound::pass(&ast, vm.environment.peek()?).context("Unbound pass in repl")?;

    let last = function_lifter::lift_functions(&ast)?;

    let llast = local::pass(&last)?;

    let code = compiler::pack_compile_lifted(&llast)?;

    vm.import_jump(&code);

    let val = vm.step_until_cost(10000)?;

    Ok(val)
}
