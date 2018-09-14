// because clippy
#![allow(unknown_lints)]
// because bench
#![feature(test)]

#[macro_use]
extern crate failure;
extern crate im;
extern crate rustyline;
extern crate test;
#[macro_use]
extern crate derive_is_enum_variant;
#[macro_use]
extern crate nom;

pub mod ast;
pub mod compiler;
pub mod data;
pub mod environment;
pub mod errors;
pub mod interpreter;
pub mod parser;
pub mod repl;
pub mod size;
pub mod syscall;
pub mod vm;

// std::usize::MAX

// TODO: keywords can't parse as starting with a number, but that produces
// N(3), :"list", separate tokens, not a parse error or a single keyword

pub fn str_to_ast(s: &str) -> errors::Result<ast::AST> {
    let p = parser::Parser::new();
    let lits = p.parse(s)?;
    let asts = ast::parse_multi(&lits)?;

    Ok(asts)
}
