//! Runtime data definitions

use im::vector::Vector;
use std::fmt;

use errors::*;

/// A data type used to represent a code location.
///
/// Most prominently used in the VM to specify and access operations in a [`Bytecode`](super::vm::bytecode::Bytecode).
/// Also used in [`LiftedAST`][super::ast::passes::function_lifter::LiftedAST] to replace lifted functions
/// with an index. Used in [`syscall`][super::syscall] to allow syscalls to emulate function calls. May
/// be used in future interpreter implementations.
pub type Address = (usize, usize);

/// Type alias for base keyword type.
///
/// `Strings` are not efficient keywords, so in theory, this alias could be used to
/// replace `Strings` with some other data structure.
pub type Keyword = String;

/// Mutate an address to increase the second value (the operation index) by 1.
pub fn address_inc(a: &mut Address) {
    a.1 += 1;
}

/// Enum representing valid runtime values for Ironic Space Lisp.
#[derive(Clone, Eq, PartialEq, is_enum_variant)]
pub enum Literal {
    Number(u32),
    Boolean(bool),
    Address(Address),
    Keyword(Keyword),
    List(Vector<Literal>),
    Closure(usize, Address),
}

/// Helper function for constructing lists [`Literal`].
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
            Literal::Keyword(k) => write!(f, ":{:}", k),
            Literal::List(ref v) => write!(f, "{:?}", v),
            Literal::Closure(arity, address) => write!(f, "{:?}/{:}", address, arity),
        }
    }
}

impl Literal {
    /// Is something truthy? Used by if expressions and [ `JumpCond` ](super::vm::op::Op)
    pub fn truthy(&self) -> bool {
        match self {
            Literal::Boolean(false) => false,
            _ => true,
        }
    }

    /// Attempt to destructure a [`Literal`] into a number, returning `Err()` if not possible.
    pub fn ensure_number(&self) -> Result<u32> {
        if let Literal::Number(n) = self {
            Ok(*n)
        } else {
            Err(format_err!("Type error, expected Number, got {:?}", self))
        }
    }

    /// Attempt to destructure a [`Literal`] into an address, returning `Err()` if not possible.
    pub fn ensure_address(&self) -> Result<Address> {
        if let Literal::Address(a) = self {
            Ok(*a)
        } else {
            Err(format_err!("Type error, expected Address, got {:?}", self))
        }
    }

    /// Attempt to destructure a [`Literal`] into a bool, returning `Err()` if not possible.
    pub fn ensure_bool(&self) -> Result<bool> {
        if let Literal::Boolean(a) = self {
            Ok(*a)
        } else {
            Err(format_err!("Type error, expected boolean, got {:?}", self))
        }
    }

    /// Attempt to destructure a [`Literal`] into a keyword, returning `Err()` if not possible.
    pub fn ensure_keyword(&self) -> Result<Keyword> {
        if let Literal::Keyword(a) = self {
            Ok(a.clone())
        } else {
            Err(format_err!("Type error, expected keyword, got {:?}", self))
        }
    }

    /// Attempt to destructure a [`Literal`] into a list, returning `Err()` if not possible.
    pub fn ensure_list(&self) -> Result<Vector<Literal>> {
        if let Literal::List(ref v) = self {
            Ok(v.clone())
        } else {
            Err(format_err!("Type error, expected list, got {:?}", self))
        }
    }

    /// Check whether a [`Literal`] can be found in this [`Literal`].
    ///
    /// Warning: I think this might be accidentally quadratic when used to
    /// check the presence of a compound [`Literal`] in another compound [`Literal`].
    /// This is because [`Vector`] equality is `O(n)` (I think, [`Vector`] is actually an RRB tree), but `contains()` subsequently
    /// iterates over those [`Vector`] elements, checking subelement equality in the
    /// same way.
    /// Non-quadratic search might not be possible unless it's restricted to
    /// finding single values rather than complex data structures, or it might be
    /// possible but very complicated to implement.
    pub fn contains(&self, p: &Literal) -> bool {
        self == p || {
            if let Literal::List(l) = self {
                l.iter().any(|prime| prime.contains(p))
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains() {
        assert!(Literal::Number(1).contains(&Literal::Number(1)));
        assert!(!Literal::Number(1).contains(&Literal::Number(2)));

        assert!(list(vec![Literal::Number(1)]).contains(&Literal::Number(1)));
        assert!(
            list(vec![
                Literal::Keyword("test".to_string()),
                Literal::Number(1)
            ]).contains(&Literal::Number(1))
        );

        assert!(
            !list(vec![list(vec![list(vec![list(vec![list(vec![])])])])])
                .contains(&Literal::Number(1))
        );
        assert!(
            list(vec![list(vec![list(vec![list(vec![list(vec![
                Literal::Number(1)
            ])])])])]).contains(&Literal::Number(1))
        );

        assert!(
            list(vec![Literal::Keyword("test".to_string())])
                .contains(&Literal::Keyword("test".to_string()))
        )
    }
}
