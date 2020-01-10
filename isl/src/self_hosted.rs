//! Contains utility functions for running the ISL implementation of ISL.
//!
//! Not very useful or reusable.

use crate::ast::ast;
use crate::compiler;
use crate::compiler::compile;
use crate::data;
use crate::env;
use crate::errors::*;
use crate::exec;
use crate::parser;
use crate::vm;
use crate::vm::bytecode;

/// Read ISL lisp implementation. See `examples/lisp.isl`.
pub fn read_lisp<'a>() -> Result<&'a str> {
    Ok(include_str!("../examples/lisp.isl"))
}

/// Read ISL's syscall test. See `examples/syscalls.isl`.
pub fn read_syscall_test<'a>() -> Result<&'a str> {
    Ok(include_str!("../examples/syscalls.isl"))
}

/// An empty [`vm::VM`] with the default libraries.
pub fn empty_vm() -> vm::VM {
    let mut builder = vm::Builder::new();

    builder.default_libs();

    builder.build()
}

fn make_double(lits: &[data::Literal], e: &env::Env) -> Result<bytecode::Bytecode> {
    let mut d = Vec::with_capacity(lits.len() + 1);
    let mut new_lits: Vec<data::Literal> = lits.to_vec();
    d.push(data::Literal::Symbol("do".to_string()));
    d.append(&mut new_lits);

    // (eval (quote (do *lits)) '())
    let caller = list_lit!("eval", list_lit!("quote", d), data::Literal::Map(ordmap![]));

    let last = ast(&[caller], e)?;
    compile(&last)
}

/// Run the ISL implementation on a [`vm::VM`], returning nothing and panicing on error.
pub fn self_hosted() -> Result<()> {
    let vm = empty_vm();

    let lits = parser::parse(&read_lisp().unwrap()).unwrap();

    let llast = ast(&lits, vm.environment.peek().unwrap()).unwrap();

    let code = compiler::compile(&llast).unwrap();

    let mut exec = exec::Exec::new();

    let (vm, res) = exec.sched(vm, &code);

    println!("{:?}", res.unwrap());

    exec.wait();

    let syscall_lits = parser::parse(&read_syscall_test().unwrap()).unwrap();

    let syscalls = make_double(&syscall_lits, vm.environment.peek().unwrap()).unwrap();

    let mut exec = exec::Exec::new();

    let (vm, res) = exec.sched(vm, &syscalls);

    println!(
        "syscalls: {:?}",
        res?.ensure_map()?
            .get(&data::Literal::Keyword("val".to_string()))
    );

    exec.wait();

    /*let double = make_double(&lits, vm.environment.peek().unwrap()).unwrap();

    let mut exec = exec::Exec::new();

    let (_, res) = exec.sched(vm, &double);

    println!(
        "hosted: {:?}",
        res?.ensure_map()?
            .get(&data::Literal::Keyword("val".to_string()))
    );

    exec.wait();*/

    Ok(())
}
