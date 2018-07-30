use std::rc::Rc;

use vm;
use errors::*;


#[derive(Debug, Clone)]
pub enum Literal {
    Number(u32),
    Builtin(Rc<vm::BuiltinFunction>),
    Lambda(Rc<vm::LambdaFunction>)
}

impl Literal {
    pub fn expect_number(&self) -> u32 {
        if let Literal::Number(n) = self {
            return *n
        } else {
            panic!("Expected number, got {:?}", self)
        }
    }

    pub fn ensure_number(&self) -> Result<u32> {
        if let Literal::Number(n) = self {
            Ok(*n)
        } else {
            Err(format!("Type error, expected Number, got {:?}", self).into())
        }
    }
}
