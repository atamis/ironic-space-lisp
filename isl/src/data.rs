//! Runtime data definitions
//!
//! For the singleton `Literal` structs (Number, Boolean, Address, Symbol, List),
//! this module implemented `From` on the base Rust data types to ease literal
//! construction.

use crate::errors::*;
#[doc(hidden)]
pub use im::vector::Vector;
#[doc(hidden)]
pub use im::OrdMap;
#[doc(hidden)]
pub use im::OrdSet;
use ordered_float::OrderedFloat;
use std::fmt;

/// A data type used to represent a code location.
///
/// Most prominently used in the VM to specify and access operations in a [`Bytecode`](super::vm::bytecode::Bytecode).
/// Also used in [`LiftedAST`][super::ast::passes::function_lifter::LiftedAST] to replace lifted functions
/// with an index. Used in [`syscall`][super::syscall] to allow syscalls to emulate function calls. May
/// be used in future interpreter implementations.
pub type Address = (usize, usize);

/// Type alias for base Symbol type.
///
/// `Strings` are not efficient Symbols, so in theory, this alias could be used to
/// replace `Strings` with some other data structure.
pub type Symbol = String;

/// Mutate an address to increase the second value (the operation index) by 1.
pub fn address_inc(a: &mut Address) {
    a.1 += 1;
}

/// Represents the address of another executing VM that can recieve messages.
#[derive(Eq, PartialEq, Clone, Copy, PartialOrd, Ord, Hash, Debug)]
pub struct Pid(pub usize);

impl Pid {
    /// Randomly generate a `Pid` from the thread local pseudorandom number generator.
    pub fn gen() -> Pid {
        use rand::prelude::*;
        Pid(thread_rng().gen())
    }
}

/// Enum representing valid runtime values for Ironic Space Lisp.
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, is_enum_variant)]
pub enum Literal {
    /// Nil, styled `nil`, representing no value.
    Nil,

    /// Boolean, styled `true` or `false`.
    Boolean(bool),

    /// A string, styled surrounded by double quotes, representing a string of
    /// characters.
    String(String),

    /// A single character.
    Char(char),

    /// A Symbol, stored as a string.
    Symbol(Symbol),

    /// A Symbol, stored as a string.
    Keyword(Symbol),

    /// Signed 64 bit number.
    Number(i64), // TODO Integer

    /// A floating point number (`f64`), wrapped in an
    /// [`OrderedFloat`](ordered_float::OrderedFloat) to support ordering.
    Float(OrderedFloat<f64>),

    /// A list, using the immutable [`Vector`](im::vector::Vector) data structure.
    List(Vector<Literal>),

    /// A vector, using the immutable [`Vector`](im::vector::Vector) data
    /// structure.
    Vector(Vector<Literal>),

    /// A map, using the immutable [`OrdMap`](im::OrdMap) data structure. Maps
    /// `Literal` to `Literal`.
    Map(OrdMap<Literal, Literal>),

    /// A set, using the immutable [`OrdSet`](im::OrdSet) data structure. Values
    /// must be unique.
    Set(OrdSet<Literal>),

    /// A tagged value, with the first argument being the tag and the second the
    /// literal itself.
    Tagged(String, Box<Literal>),

    /// An `[Address]`, or a tuple of 2 `usize`, representing an executable block of code.
    Address(Address),

    /// A closure, an [`Address`] that includes an arity.
    Closure(usize, Address),

    /// A [`Pid`], representing another executing [`super::vm::VM`] that can recieve messages.
    Pid(Pid),
}

/// Helper function for constructing lists [`Literal`].
pub fn list(v: Vec<Literal>) -> Literal {
    Literal::List(v.into_iter().collect())
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Nil => write!(f, "nil"),
            Literal::Boolean(true) => write!(f, "true"),
            Literal::Boolean(false) => write!(f, "false"),
            Literal::String(s) => write!(f, "{:?}", s),
            Literal::Char(c) => write!(f, "\\{}", c),
            Literal::Number(n) => write!(f, "N({:?})", n),
            Literal::Float(fl) => write!(f, "F({:?})", fl.into_inner()),
            Literal::Address(a) => write!(f, "A({:?})", a),
            Literal::Symbol(s) => write!(f, "{:}", s),
            Literal::Keyword(k) => write!(f, ":{:}", k),
            Literal::List(ref v) => {
                write!(f, "(")?;

                for (idx, l) in v.iter().enumerate() {
                    write!(f, "{:?}", l)?;
                    if idx != v.len() - 1 {
                        write!(f, " ")?;
                    }
                }

                write!(f, ")")
            }
            Literal::Vector(ref v) => {
                write!(f, "[")?;

                for (idx, l) in v.iter().enumerate() {
                    write!(f, "{:?}", l)?;
                    if idx != v.len() - 1 {
                        write!(f, " ")?;
                    }
                }

                write!(f, "]")
            }
            Literal::Map(ref m) => {
                write!(f, "{{")?;

                for (idx, (k, v)) in m.iter().enumerate() {
                    write!(f, "{:?} {:?}", k, v)?;

                    if idx != m.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")
            }
            Literal::Set(ref s) => {
                write!(f, "#{{")?;

                for (idx, k) in s.iter().enumerate() {
                    write!(f, "{:?}", k)?;

                    if idx != s.len() - 1 {
                        write!(f, " ")?;
                    }
                }

                write!(f, "}}")
            }
            Literal::Tagged(t, v) => write!(f, "#{} {:?}", t, v),
            Literal::Closure(arity, address) => write!(f, "{:?}/{:}", address, arity),
            Literal::Pid(Pid(n)) => write!(f, "<{}>", n),
        }
    }
}

impl Literal {
    /// Make a new Symbol from something that can be turned into a `String`
    pub fn new_symbol<T>(s: T) -> Literal
    where
        T: Into<String>,
    {
        Literal::Symbol(s.into())
    }

    /// Is something truthy? Used by if expressions and [ `JumpCond` ](super::vm::op::Op)
    pub fn truthy(&self) -> bool {
        match self {
            Literal::Nil => false,
            Literal::Boolean(false) => false,
            _ => true,
        }
    }

    /// Attempt to destructure a [`Literal`] into a number, returning `Err()` if not possible.
    pub fn ensure_number(&self) -> Result<i64> {
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

    /// Attempt to destructure a [`Literal`] into a Symbol, returning `Err()` if not possible.
    pub fn ensure_symbol(&self) -> Result<Symbol> {
        if let Literal::Symbol(a) = self {
            Ok(a.clone())
        } else {
            Err(format_err!("Type error, expected Symbol, got {:?}", self))
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

    /// Attempt to destructure a [`Literal`] into a vector, returning `Err()` if not possible.
    pub fn ensure_vector(&self) -> Result<Vector<Literal>> {
        if let Literal::Vector(ref v) = self {
            Ok(v.clone())
        } else {
            Err(format_err!("Type error, expected vector, got {:?}", self))
        }
    }

    /// Attempt to destructure a [`Literal`] into a [`Pid`], returning `Err()` if not possible.
    pub fn ensure_pid(&self) -> Result<Pid> {
        if let Literal::Pid(n) = self {
            Ok(*n)
        } else {
            Err(format_err!("Type error, expected pid, got {:?}", self))
        }
    }

    /// Attempt to destructure a [`Literal`] into a tuple of two literals.
    ///
    /// Converts 2 element lists and vectors to a tuple of 2 literals.
    pub fn ensure_pair(&self) -> Result<(Literal, Literal)> {
        // TODO maps
        let v = match self {
            Literal::List(ref v) => Ok(v),
            Literal::Vector(ref v) => Ok(v),
            x => Err(err_msg(format!(
                "Type error, expected pair ((a b), [a b]), got {:?}",
                x
            ))),
        }?;

        if v.len() != 2 {
            Err(err_msg(format!(
                "Type error, expected pair, but len was {}",
                v.len()
            )))
        } else {
            Ok((v.get(0).unwrap().clone(), v.get(1).unwrap().clone()))
        }
    }

    /// Attempt to destructure a [`Literal`] into a map, returning an error otherwise.
    pub fn ensure_map(&self) -> Result<OrdMap<Literal, Literal>> {
        if let Literal::Map(ref m) = self {
            Ok(m.clone())
        } else {
            Err(err_msg(format!("Type error, expected map, got {:?}", self)))
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

impl From<i64> for Literal {
    fn from(n: i64) -> Literal {
        Literal::Number(n)
    }
}

impl From<String> for Literal {
    fn from(s: String) -> Literal {
        Literal::new_symbol(s)
    }
}

impl<'a> From<&'a str> for Literal {
    fn from(s: &str) -> Literal {
        Literal::new_symbol(s)
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

impl From<Pid> for Literal {
    fn from(p: Pid) -> Literal {
        Literal::Pid(p)
    }
}

impl From<OrdMap<Literal, Literal>> for Literal {
    fn from(m: OrdMap<Literal, Literal>) -> Literal {
        Literal::Map(m)
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
        assert!(list(vec![
            Literal::Symbol("test".to_string()),
            Literal::Number(1)
        ])
        .contains(&Literal::Number(1)));

        assert!(
            !list(vec![list(vec![list(vec![list(vec![list(vec![])])])])])
                .contains(&Literal::Number(1))
        );
        assert!(list(vec![list(vec![list(vec![list(vec![list(vec![
            Literal::Number(1)
        ])])])])])
        .contains(&Literal::Number(1)));

        assert!(list(vec![Literal::Symbol("test".to_string())])
            .contains(&Literal::Symbol("test".to_string())))
    }

    #[test]
    fn test_from() {
        let a1: Literal = (1 as i64).into();
        assert_eq!(a1, Literal::Number(1));

        let a2: Literal = "test".into();
        assert_eq!(a2, Literal::Symbol("test".to_string()));

        let a3: Literal = ("test".to_string()).into();
        assert_eq!(a3, Literal::Symbol("test".to_string()));

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

    #[test]
    fn test_ensure_pair() {
        assert_eq!(list_lit![1, 2].ensure_pair().unwrap(), (1.into(), 2.into()));
        assert_eq!(
            Literal::Vector(vector![1.into(), 2.into()])
                .ensure_pair()
                .unwrap(),
            (1.into(), 2.into())
        );
    }
}
