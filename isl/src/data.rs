//! Runtime data definitions
//!
//! For the singleton `Literal` structs (Number, Boolean, Address, keyword, List),
//! this module implemented `From` on the base Rust data types to ease literal
//! construction.

use errors::*;
#[doc(hidden)]
pub use im::vector::Vector;
use std::fmt;

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
    pub fn new_keyword<T: Into<String>>(s: T) -> Literal {
        Literal::Keyword(s.into())
    }

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

    /// Attempt to destructure a [`Literal`] into an address, but flexibly, returning `Err()` if not possible.
    ///
    /// Will destructure both [`Literal::Address`] and [`Literal::Closure`], throwing away arity information.
    pub fn ensure_address_flexible(&self) -> Result<Address> {
        match self {
            Literal::Address(a) => Ok(*a),
            Literal::Closure(_arity, addr) => Ok(*addr),
            _ => Err(format_err!(
                "Type error, expected Address or Closure, got {:?}",
                self
            )),
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

impl From<u32> for Literal {
    fn from(n: u32) -> Literal {
        Literal::Number(n)
    }
}

impl From<String> for Literal {
    fn from(s: String) -> Literal {
        Literal::new_keyword(s)
    }
}

impl<'a> From<&'a str> for Literal {
    fn from(s: &str) -> Literal {
        Literal::new_keyword(s)
    }
}

impl From<bool> for Literal {
    fn from(b: bool) -> Literal {
        Literal::Boolean(b)
    }
}

impl From<Address> for Literal {
    fn from(a: Address) -> Literal {
        Literal::Address(a)
    }
}

impl From<Vector<Literal>> for Literal {
    fn from(v: Vector<Literal>) -> Literal {
        Literal::List(v)
    }
}

impl From<Vec<Literal>> for Literal {
    fn from(v: Vec<Literal>) -> Literal {
        list(v)
    }
}

/// Macro to easily make a [`Literal::List`](data::Literal::List).
///
/// ```
/// # #[macro_use] extern crate isl;
/// list_lit![];
/// list_lit![1, 2, 3];
/// list_lit![1, 2, 3,];
/// ```
///
/// Calls `into()` on all elements passed in so that they'll be converted
/// to [`Literal`](data::Literal). This may result in a rather cryptic
/// type error, or missing trait error, if you put an element in that can't be
/// converted to a [`Literal`](data::Literal).
#[macro_export]
macro_rules! list_lit {
    () => {
       $crate::data::Literal::List($crate::data::Vector::new())
    };

    ( $($x:expr),* ) => {{
        let mut v = $crate::data::Vector::new();
        $(
            v.push_back($x.into());
        )*
        let l: $crate::data::Literal = v.into();
        l
    }};

    ( $($x:expr, )* ) => {{
        let mut v = $crate::data::Vector::new();
        $(
            v.push_back($x.into());
        )*
            let l: $crate::data::Literal = v.into();
        l
    }};
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
            ])
            .contains(&Literal::Number(1))
        );

        assert!(
            !list(vec![list(vec![list(vec![list(vec![list(vec![])])])])])
                .contains(&Literal::Number(1))
        );
        assert!(
            list(vec![list(vec![list(vec![list(vec![list(vec![
                Literal::Number(1)
            ])])])])])
            .contains(&Literal::Number(1))
        );

        assert!(
            list(vec![Literal::Keyword("test".to_string())])
                .contains(&Literal::Keyword("test".to_string()))
        )
    }

    #[test]
    fn test_from() {
        let a1: Literal = (1 as u32).into();
        assert_eq!(a1, Literal::Number(1));

        let a2: Literal = "test".into();
        assert_eq!(a2, Literal::Keyword("test".to_string()));

        let a3: Literal = ("test".to_string()).into();
        assert_eq!(a3, Literal::Keyword("test".to_string()));

        let a4: Literal = true.into();
        assert_eq!(a4, Literal::Boolean(true));

        let a5: Literal = false.into();
        assert_eq!(a5, Literal::Boolean(false));

        let a6: Literal = (2, 3).into();
        assert_eq!(a6, Literal::Address((2, 3)));

        let a7: Literal = vector![1.into(), 2.into()].into();
        assert_eq!(a7, list(vec![Literal::Number(1), Literal::Number(2)]))
    }

    #[test]
    fn test_lit_list() {
        assert_eq!(list_lit![], list(vec![]));
        assert_eq!(list_lit![1, 2, 3], list(vec![1.into(), 2.into(), 3.into()]));
        assert_eq!(
            list_lit![1, 2, 3,],
            list(vec![1.into(), 2.into(), 3.into()])
        );
    }
}
