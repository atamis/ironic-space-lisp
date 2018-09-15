extern crate clap;
extern crate ironic_space_lisp;

use clap::{App, SubCommand};
use ironic_space_lisp::errors::*;
use ironic_space_lisp::repl;
use ironic_space_lisp::size::DataSize;

use std::io::prelude::*;

fn read_stdin() -> Result<String> {
    use std::io;
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    Ok(buffer)
}

/*
let mut f = File::open(filename).context("file not found")?;

let mut contents = String::new();
f.read_to_string(&mut contents)
.context("something went wrong reading the file")?;
*/

fn exec(content: &str) -> Result<()> {
    {
        use ironic_space_lisp::ast;
        use ironic_space_lisp::ast::passes::function_lifter;
        use ironic_space_lisp::ast::passes::list;
        use ironic_space_lisp::ast::passes::unbound;
        use ironic_space_lisp::compiler;
        use ironic_space_lisp::parser;
        use ironic_space_lisp::vm;
        let mut vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));

        let p = parser::Parser::new();

        let lits = p.parse(&content).context("While parsing contents")?;

        let ast = ast::parse_multi(&lits).context("While ast parsing literals")?;

        let ast = list::pass(&ast)?;

        unbound::pass(&ast, vm.environment.peek()?)?;

        let last = function_lifter::lift_functions(&ast).context("While lifting functions")?;

        let code = compiler::pack_compile_lifted(&last).context("Packing lifted ast")?;

        vm.import_jump(&code);
        let res = vm.step_until_value(false);

        match res {
            Ok(x) => println!("{:#?}", x),
            Err(e) => {
                vm.code.dissassemble();
                println!("{:#?}", vm);
                return Err(e);
            }
        }
    }

    Ok(())
}

fn inspect(content: &str) -> Result<()> {
    println!("Code:\n {:}", content);

    {
        use ironic_space_lisp::ast;
        use ironic_space_lisp::ast::passes::function_lifter;
        use ironic_space_lisp::ast::passes::list;
        use ironic_space_lisp::ast::passes::unbound;
        use ironic_space_lisp::compiler;
        use ironic_space_lisp::parser;
        use ironic_space_lisp::vm;

        let vm = vm::VM::new(vm::bytecode::Bytecode::new(vec![]));

        let p = parser::Parser::new();

        let lits = p.parse(&content).context("While parsing contents")?;

        println!("Literal size: {:}", lits.data_size());
        println!("Literals: {:#?}", lits);

        let ast = ast::parse_multi(&lits).context("While ast parsing literals")?;

        println!("AST: {:#?}", ast);

        let list_ast = list::pass(&ast)?;

        println!("Applying list pass, ASTs equal? {:}", list_ast == ast);

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

        let code = compiler::pack_compile_lifted(&last).context("Packing lifted ast")?;

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
        .get_matches();

    match matches.subcommand() {
        ("inspect", Some(_inspect_matches)) => {
            inspect(&read_stdin()?).context("While inspecting")?;
        }
        ("run", Some(_run_matches)) => {
            exec(&read_stdin()?).context("While executing")?;
        }
        _ => {
            println!("Booting repl");

            repl::repl();
        }
    }

    Ok(())
}
