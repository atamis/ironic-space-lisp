//! Contains utility functions for running the ISL implementation of ISL.
//!
//! Not very useful or reusable.

use crate::ast::ast;
use crate::ast::LiftedAST;
use crate::compiler;
use crate::data;
use crate::env;
use crate::errors::*;
use crate::parser;
use crate::vm;
use crate::vm::bytecode;
use ast::passes::local;
use compiler::pack_compile_lifted;
use errors::*;

/// Read ISL lisp implementation. See `examples/lisp.isl`.
pub fn read_lisp<'a>() -> Result<&'a str> {
    Ok(include_str!("../examples/lisp.isl"))
}

fn compile(last: &LiftedAST) -> Result<bytecode::Bytecode> {
    let llast = local::pass(&last).unwrap();
    compiler::pack_compile_lifted(&llast)
}

/// An empty [`vm::VM`] with the default libraries.
pub fn empty_vm() -> vm::VM {
    let mut builder = vm::Builder::new();

    builder.default_libs();

    builder.build()
}

fn make_double(lits: &[data::Literal], e: &env::Env) -> Result<bytecode::Bytecode> {
    let mut d = Vec::with_capacity(lits.len() + 1);
    let mut new_lits: Vec<data::Literal> = lits.into_iter().cloned().collect();
    d.push(data::Literal::Keyword("do".to_string()));
    d.append(&mut new_lits);

    // (eval (quote (do *lits)) '())
    let caller = list_lit!(
        "eval",
        list_lit!("quote", d.clone()),
        list_lit!("quote", list_lit!())
    );

    let last = ast(&[caller], e)?;
    pack_compile_lifted(&last)
}

/// Run the ISL implementation on a [`vm::VM`], returning nothing and panicing on error.
pub fn self_hosted() -> Result<()> {
    let mut vm = empty_vm();

    let s = read_lisp().unwrap();

    let lits = parser::parse(&s).unwrap();

    let llast = ast(&lits, vm.environment.peek().unwrap()).unwrap();

    let code = compiler::pack_compile_lifted(&llast).unwrap();

    vm.import_jump(&code);

    println!("{:?}", vm.step_until_value().unwrap());

    let double = make_double(&lits, vm.environment.peek().unwrap()).unwrap();

    vm.import_jump(&double);

    println!("hosted: {:?}", vm.step_until_value()?.ensure_list()?[1]);

    Ok(())
}
