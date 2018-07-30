use std::rc::Rc;
use std::fmt;

/// Omni-datatype. Represents both data and code for the lisp VM.
#[derive(Clone, PartialEq)]
pub enum Lisp {
    /// Represents a single u32 number.
    Num(u32),
    /// Represents an operation see `Op` for more info.
    Op(Op),
    /// Represents a list of `Lisp` values. Note that this is reference counted.
    List(Rc<Vec<Rc<Lisp>>>),
}

impl fmt::Debug for Lisp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Lisp::Num(i) => write!(f, "{:?}", i),
            Lisp::Op(o) => write!(f, "{:?}", o),
            Lisp::List(l) => write!(f, "{:?}", l),
        }
    }
}

/// Enum of basic operations.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Represents addition. Currently variadic.
    Add,
    If,
}

/// Simplified method of making a list of lisp datums. Sets up both the
/// Rc and tags it with the enum.
pub fn make_list(mut items: Vec<Lisp>) -> Lisp {
    let mut rcs = Vec::with_capacity(items.len());

    while items.len() > 0 {
        // TODO: not this?
        rcs.push(Rc::new(items.remove(0)));
    }

    Lisp::List(Rc::new(rcs))
}
