use std::usize;

use ::data;
use ::errors::*;

pub type BuiltinFn = Fn(&mut Vec<data::Literal>) -> ();

#[derive(Debug)]
pub struct Builtin;

impl Builtin {
    pub fn new() -> Builtin {
        Builtin
    }

    pub fn lookup(&self, chunk: usize) -> Option<Box<BuiltinFn>> {
        let c = usize::MAX - chunk;

        if c == 0 {
            return Some(Box::new(Builtin::add));
        }

        None
    }

    fn add(s: &mut Vec<data::Literal>) {
    }

}
