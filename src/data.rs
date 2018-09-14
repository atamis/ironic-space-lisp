use im::vector::Vector;
use std::fmt;

use errors::*;

pub type Address = (usize, usize);
pub type Keyword = String;

pub fn address_inc(a: &mut Address) {
    a.1 += 1;
}

#[derive(Clone, Eq, PartialEq, is_enum_variant)]
pub enum Literal {
    Number(u32),
    Boolean(bool),
    Address(Address),
    Keyword(Keyword),
    List(Vector<Literal>),
    Closure(usize, Address),
}

pub fn list(v: Vec<Literal>) -> Literal {
    Literal::List(v.into_iter().collect())
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
            Literal::Closure(arity, address) => write!(f, "{:?}/{:}", address, arity),
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
            Err(format_err!("Type error, expected Number, got {:?}", self))
        }
    }

    pub fn ensure_address(&self) -> Result<Address> {
        if let Literal::Address(a) = self {
            Ok(*a)
        } else {
            Err(format_err!("Type error, expected Address, got {:?}", self))
        }
    }

    pub fn ensure_bool(&self) -> Result<bool> {
        if let Literal::Boolean(a) = self {
            Ok(*a)
        } else {
            Err(format_err!("Type error, expected boolean, got {:?}", self))
        }
    }

    pub fn ensure_keyword(&self) -> Result<Keyword> {
        if let Literal::Keyword(a) = self {
            Ok(a.clone())
        } else {
            Err(format_err!("Type error, expected keyword, got {:?}", self))
        }
    }

    pub fn ensure_list(&self) -> Result<Vector<Literal>> {
        if let Literal::List(ref v) = self {
            Ok(v.clone())
        } else {
            Err(format_err!("Type error, expected list, got {:?}", self))
        }
    }
}
