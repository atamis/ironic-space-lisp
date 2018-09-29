// because clippy
#![allow(unknown_lints)]
#![feature(tool_lints)]
// because bench
#![feature(test)]

#[macro_use]
extern crate failure;
#[allow(unused_imports)]
#[macro_use]
extern crate im;
extern crate rustyline;
extern crate test;
#[macro_use]
extern crate derive_is_enum_variant;
#[macro_use]
extern crate nom;
extern crate futures;
extern crate rand;
extern crate tokio;
extern crate tokio_channel;

pub mod ast;
pub mod compiler;
#[macro_use]
pub mod data;
pub mod env;
pub mod errors;
pub mod exec;
pub mod interpreter;
pub mod parser;
pub mod repl;
pub mod self_hosted;
pub mod size;
pub mod syscall;
mod util;
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
