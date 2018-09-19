extern crate ironic_space_lisp;

use std::io::prelude::*;

use ironic_space_lisp::ast;
use ironic_space_lisp::ast::passes::function_lifter;
use ironic_space_lisp::ast::passes::internal_macro;
use ironic_space_lisp::ast::passes::unbound;
use ironic_space_lisp::compiler;
use ironic_space_lisp::data;
use ironic_space_lisp::environment;
use ironic_space_lisp::errors::*;
use ironic_space_lisp::parser;
use ironic_space_lisp::vm;
use ironic_space_lisp::vm::bytecode;

// from src/main.rs
fn read_lisp() -> Result<String> {
    use std::fs::File;
    let mut f = File::open("examples/lisp.isl").context("file not found")?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .context("something went wrong reading the file")?;

    Ok(contents)
}

fn ast(lits: &[data::Literal], e: &environment::Env) -> Result<function_lifter::LiftedAST> {
    let ast = ast::parse_multi(&lits)?;
    let ast = internal_macro::pass(&ast)?;

    unbound::pass(&ast, e)?;

    let last = function_lifter::lift_functions(&ast)?;

    Ok(last)
}

fn compile(last: &function_lifter::LiftedAST) -> Result<bytecode::Bytecode> {
    compiler::pack_compile_lifted(&last)
}

fn empty_vm() -> vm::VM {
    let mut builder = vm::Builder::new();

    builder.default_libs();

    builder.build()
}

fn make_double(lits: &[data::Literal], e: &environment::Env) -> Result<bytecode::Bytecode> {
    let mut d = Vec::with_capacity(lits.len() + 1);
    let mut new_lits: Vec<data::Literal> = lits.into_iter().cloned().collect();
    d.push(data::Literal::Keyword("do".to_string()));
    d.append(&mut new_lits);

    // (eval (quote (do *lits)) '())
    let caller = data::list(vec![
        data::Literal::Keyword("eval".to_string()),
        data::list(vec![
            data::Literal::Keyword("quote".to_string()),
            data::list(d),
        ]),
        data::list(vec![
            data::Literal::Keyword("quote".to_string()),
            data::list(vec![]),
        ]),
    ]);

    let last = ast(&[caller], e)?;
    compile(&last)
}

#[test]
fn test_double() {
    let mut vm = empty_vm();

    let s = read_lisp().unwrap();

    let lits = parser::parse(&s).unwrap();

    let last = ast(&lits, vm.environment.peek().unwrap()).unwrap();

    let code = compiler::pack_compile_lifted(&last).unwrap();

    vm.import_jump(&code);

    println!("{:?}", vm.step_until_value(false).unwrap());

    let double = make_double(&lits, vm.environment.peek().unwrap()).unwrap();

    vm.import_jump(&double);

    vm.code.dissassemble();

    println!("{:?}", vm.step_until_value(false).unwrap());

    assert!(false);
}
