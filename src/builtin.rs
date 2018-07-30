use std::usize;

use data::Literal;
use errors::*;

pub type BuiltinFn = Fn(&mut Vec<Literal>) -> Result<()>;

#[derive(Debug)]
pub struct Builtin;

// https://github.com/sfackler/rust-phf
// Use this for easier/faster name-address-function lookups.
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

    fn add(s: &mut Vec<Literal>) -> Result<()> {
        let a = s
            .pop()
            .ok_or("Popping number for add builtin")?
            .ensure_number()?;
        let b = s
            .pop()
            .ok_or("Popping number for add builtin")?
            .ensure_number()?;

        s.push(Literal::Number(a + b));

        Ok(())
    }
}
