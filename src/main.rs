extern crate clap;
extern crate ironic_space_lisp;

use clap::{App, Arg, SubCommand};
use ironic_space_lisp::errors::*;
use ironic_space_lisp::repl;
use ironic_space_lisp::size::DataSize;

use std::fs::File;
use std::io::prelude::*;

fn inspect(filename: &str) -> Result<()> {
    let mut f = File::open(filename).context("file not found")?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("something went wrong reading the file")?;

    println!("Code:\n {:}", contents);

    {
        use ironic_space_lisp::ast;
        use ironic_space_lisp::ast::passes::function_lifter;
        use ironic_space_lisp::ast::passes::list;
        use ironic_space_lisp::ast::passes::unbound;
        use ironic_space_lisp::compiler;
        use ironic_space_lisp::parser;
        use ironic_space_lisp::vm;

        let vm = vm::VM::new(vm::Bytecode::new(vec![]));

        let p = parser::Parser::new();

        let lits = p.parse(&contents).context("While parsing contents")?;

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
        .subcommand(
            SubCommand::with_name("inspect")
                .about("Inspect the parsing of some ISL code")
                .arg(Arg::with_name("file").required(true)),
        ).get_matches();

    match matches.subcommand() {
        ("inspect", Some(inspect_matches)) => match inspect_matches.value_of("file") {
            Some(filename) => {
                inspect(filename).context(format!("While inspecting {:}", filename))?;
            }
            None => unreachable!(),
        },
        _ => {
            println!("Booting repl");

            repl::repl();
        }
    }

    Ok(())
}
