// because clippy
#![allow(unknown_lints)]

#[macro_use]
extern crate failure;
extern crate im;
extern crate lalrpop_util;

pub mod ast;
pub mod builtin;
pub mod compiler;
pub mod data;
pub mod environment;
pub mod errors;
pub mod interpreter;
pub mod parser;
pub mod vm;

// std::usize::MAX

pub fn str_to_ast(s: &str) -> errors::Result<Vec<ast::AST>> {
    use errors::*;

    let p = parser::Parser::new();
    let lits = p.parse(s)?;
    let asts: Vec<ast::AST> = lits
        .iter()
        .enumerate()
        .map(|(i, lit)| {
            let a = ast::parse(&lit).context(format!("While parsing literal #{:}", i))?;
            Ok(a)
        }).collect::<Result<_>>()?;
    ast::passes::unbound::pass_default(asts.as_ref())?;
    Ok(asts)
}
