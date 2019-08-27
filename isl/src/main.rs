extern crate clap;
extern crate isl;

use clap::{App, SubCommand};
use isl::errors::*;
use isl::repl;
use isl::self_hosted;
use isl::size::DataSize;

use std::io::prelude::*;

fn read_stdin() -> Result<String> {
    use std::io;
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    Ok(buffer)
}

fn exec(content: &str) -> Result<()> {
    {
        use isl::ast;
        use isl::ast::passes::function_lifter;
        use isl::ast::passes::internal_macro;
        use isl::ast::passes::local;
        use isl::ast::passes::unbound;
        use isl::compiler;
        use isl::exec;
        use isl::parser;
        use isl::vm;
        let vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));

        let p = parser::Parser::new();

        let lits = p.parse(&content).context("While parsing contents")?;

        let ast = ast::parse_multi(&lits).context("While ast parsing literals")?;

        let ast = internal_macro::pass(&ast)?;

        unbound::pass(&ast, vm.environment.peek()?)?;

        let last = function_lifter::lift_functions(&ast).context("While lifting functions")?;

        let llast = local::pass(&last).context("While local pass")?;

        let code = compiler::compile(&llast).context("Packing lifted ast")?;

        let mut exec = exec::Exec::new();

        let (vm, res) = exec.sched(vm, &code);

        println!("{:?}", (&vm, &res));

        match res {
            Ok(x) => println!("{:#?}", x),
            Err(e) => {
                vm.code.dissassemble();
                println!("{:#?}", vm);
                return Err(e);
            }
        }

        exec.wait();
    }

    Ok(())
}

fn inspect(content: &str) -> Result<()> {
    println!("Code:\n {:}", content);

    {
        use isl::ast;
        use isl::ast::passes::function_lifter;
        use isl::ast::passes::internal_macro;
        use isl::ast::passes::local;
        use isl::ast::passes::unbound;
        use isl::compiler;
        use isl::parser;
        use isl::vm;

        let vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));

        let p = parser::Parser::new();

        let lits = p.parse(&content).context("While parsing contents")?;

        println!("Literal size: {:}", lits.data_size());
        println!("Literals: {:#?}", lits);

        let ast = ast::parse_multi(&lits).context("While ast parsing literals")?;

        println!("AST: {:#?}", ast);

        let list_ast = internal_macro::pass(&ast)?;

        println!("Applying macro pass, ASTs equal? {:}", list_ast == ast);

        let ast = list_ast;

        if let Err(ref e) = unbound::pass(&ast, vm.environment.peek()?) {
            println!("While in unbound pass");
            println!("error: {}", e);

            for e in e.iter_causes() {
                println!("caused by: {}", e);
            }
        } else {
            println!("Unbound pass successful")
        }

        let last = function_lifter::lift_functions(&ast).context("While lifting functions")?;

        println!("LAST: {:#?}", last);

        let llast = local::pass(&last).context("While local pass")?;

        println!("LLAST: {:#?}", llast);

        let code = compiler::compile(&llast).context("While compiling")?;

        code.dissassemble();
    }

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter_causes() {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = Some(e.backtrace()) {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let matches = App::new("ironic-space-lisp")
        .version("v0.1.0")
        .author("Andrew Amis <atamiser@gmail.com>")
        .about("Rust implementation of the Ironic Space Lisp runtime")
        .subcommand(SubCommand::with_name("repl").about("Live read and evaluate ISL"))
        .subcommand(SubCommand::with_name("inspect").about("Inspect the parsing of some ISL code"))
        .subcommand(SubCommand::with_name("run").about("Run input"))
        .subcommand(SubCommand::with_name("self").about("Run self-hosted interpreter."))
        .get_matches();

    match matches.subcommand() {
        ("inspect", Some(_inspect_matches)) => {
            inspect(&read_stdin()?).context("While inspecting")?;
        }
        ("run", Some(_run_matches)) => {
            exec(&read_stdin()?).context("While executing")?;
        }
        ("self", Some(_self_matches)) => {
            self_hosted::self_hosted().context("Executing self-hosted interpreter")?;
        }
        _ => {
            println!("Booting repl");

            repl::repl();
        }
    }

    Ok(())
}
