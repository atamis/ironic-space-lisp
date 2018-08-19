use std::fmt;
use std::rc::Rc;

use errors::*;

pub type Address = (usize, usize);
pub type Keyword = String;

pub fn address_inc(a: &mut Address) {
    a.1 += 1;
}

#[derive(Clone, Eq, PartialEq)]
pub enum Literal {
    Number(u32),
    Boolean(bool),
    Address(Address),
    Keyword(Keyword),
    List(Rc<Vec<Literal>>),
}

pub fn list(v: Vec<Literal>) -> Literal {
    Literal::List(Rc::new(v))
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "N({:?})", n),
            Literal::Boolean(true) => write!(f, "#t"),
            Literal::Boolean(false) => write!(f, "#f"),
            Literal::Address(a) => write!(f, "A({:?})", a),
            Literal::Keyword(k) => write!(f, ":{:?}", k),
            Literal::List(ref v) => write!(f, "{:?}", v),
        }
    }
}

impl Literal {

    pub fn truthy(&self) -> bool {
        match self {
            Literal::Boolean(false) => false,
            _ => true,
        }
    }

    pub fn ensure_number(&self) -> Result<u32> {
        if let Literal::Number(n) = self {
            Ok(*n)
        } else {
            Err(format!("Type error, expected Number, got {:?}", self).into())
        }
    }

    pub fn ensure_address(&self) -> Result<Address> {
        if let Literal::Address(a) = self {
            Ok(*a)
        } else {
            Err(format!("Type error, expected Address, got {:?}", self).into())
        }
    }

    pub fn ensure_bool(&self) -> Result<bool> {
        if let Literal::Boolean(a) = self {
            Ok(*a)
        } else {
            Err(format!("Type error, expected boolean, got {:?}", self).into())
        }
    }

    pub fn ensure_keyword(&self) -> Result<Keyword> {
        if let Literal::Keyword(a) = self {
            Ok(a.clone())
        } else {
            Err(format!("Type error, expected keyword, got {:?}", self).into())
        }
    }

    pub fn ensure_list(&self) -> Result<Rc<Vec<Literal>>> {
        if let Literal::List(ref v) = self {
            Ok(Rc::clone(v))
        } else {
            Err(format!("Type error, expected list, got {:?}", self).into())
        }
    }
}
