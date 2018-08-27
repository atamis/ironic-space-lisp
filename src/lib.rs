// because clippy
#![allow(unknown_lints)]
// because bench
#![feature(test)]

#[macro_use]
extern crate failure;
extern crate im;
extern crate lalrpop_util;
extern crate rustyline;
extern crate test;

pub mod ast;
pub mod builtin;
pub mod compiler;
pub mod data;
pub mod environment;
pub mod errors;
pub mod interpreter;
pub mod parser;
pub mod repl;
pub mod vm;

// std::usize::MAX

pub fn str_to_ast(s: &str) -> errors::Result<ast::AST> {
    let p = parser::Parser::new();
    let lits = p.parse(s)?;
    let asts = ast::parse_multi(&lits)?;

    Ok(asts)
}
