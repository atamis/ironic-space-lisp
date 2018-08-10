use errors::*;

pub type Address = (usize, usize);

pub fn address_inc(a: &mut Address) {
    a.1 += 1;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Literal {
    Number(u32),
    Boolean(bool),
    Address(Address),
}

impl Literal {
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
}
