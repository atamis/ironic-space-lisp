use std::io::prelude::*;

use ast;
use ast::passes::function_lifter;
use ast::passes::internal_macro;
use ast::passes::unbound;
use compiler;
use data;
use environment;
use errors::*;
use parser;
use vm;
use vm::bytecode;

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
    let caller = list_lit!(
        "eval",
        list_lit!("quote", d.clone()),
        list_lit!("quote", list_lit!())
    );

    let last = ast(&[caller], e)?;
    compile(&last)
}

pub fn self_hosted() -> Result<()> {
    let mut vm = empty_vm();

    let s = read_lisp().unwrap();

    let lits = parser::parse(&s).unwrap();

    let last = ast(&lits, vm.environment.peek().unwrap()).unwrap();

    let code = compiler::pack_compile_lifted(&last).unwrap();

    vm.import_jump(&code);

    println!("{:?}", vm.step_until_value(false).unwrap());

    let double = make_double(&lits, vm.environment.peek().unwrap()).unwrap();

    vm.import_jump(&double);

    println!(
        "hosted: {:?}",
        vm.step_until_value(false)?.ensure_list()?[1]
    );

    Ok(())
}
