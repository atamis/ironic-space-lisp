//! Run an interactive REPL on a [`vm::VM`].

use rustyline::error::ReadlineError;
use rustyline::Editor;

use crate::ast::passes::function_lifter;
use crate::ast::passes::internal_macro;
use crate::ast::passes::local;
use crate::ast::passes::unbound;
use crate::compiler;
use crate::errors::*;
use crate::exec;
use crate::size::*;
use crate::str_to_ast;
use crate::vm;
use crate::vm::bytecode;

/// Run a REPL executing on a [`vm::VM`].
pub fn repl() {
    let mut vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));
    let mut exec = exec::Exec::new();
    vm.proc = Some(Box::new(exec.get_handle()));

    let mut rl = Editor::<()>::new();

    loop {
        let readline = rl.readline(&format!("{:} {:?} >", vm.code.chunks.len(), vm.data_size()));

        let line = match readline {
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("Error encountered in repl: {:?}", e);
                break;
            }
            Ok(s) => s,
        };

        rl.add_history_entry(&line);

        let code = compile(&mut vm, &line);

        if let Err(e) = code {
            eprintln!("Error encountered in compiler: {:?}", e);
            for e in e.iter_causes() {
                println!("caused by: {}", e);
            }
            continue;
        }

        let code = code.unwrap();

        let (new_vm, res) = exec.sched(vm, &code);

        vm = new_vm;

        match res {
            Err(ref e) => {
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
            }
            Ok(v) => println!("{:?}", v),
        }
    }
}

/// Parse a string and evaluate it on the VM with a limited resource pool of 10000 cost.
pub fn compile(vm: &mut vm::VM, s: &str) -> Result<bytecode::Bytecode> {
    let ast = str_to_ast(&s)?;

    let ast = internal_macro::pass(&ast)?;

    unbound::pass(&ast, vm.environment.peek()?).context("Unbound pass in repl")?;

    let last = function_lifter::lift_functions(&ast)?;

    let llast = local::pass(&last)?;

    let code = compiler::pack_compile_lifted(&llast)?;

    //vm.import_jump(&code);

    //let val = vm.step_until_cost(10000)?;

    Ok(code)
}
