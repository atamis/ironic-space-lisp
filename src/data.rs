use std::rc::Rc;

/// Omni-datatype. Represents both data and code for the lisp VM.
#[derive(Debug, Clone)]
pub enum Lisp {
    /// Represents a single u32 number.
    Num(u32),
    /// Represents an operation see `Op` for more info.
    Op(Op),
    /// Represents a list of `Lisp` values. Note that this is reference counted.
    List(Rc<Vec<Lisp>>),
}

/// Enum of basic operations.
#[derive(Debug, Clone)]
pub enum Op {
    /// Represents addition. Currently variadic.
    Add,
}

/// Simplified method of making a list of lisp datums. Sets up both the
/// Rc and tags it with the enum.
pub fn make_list<'a>(items: Vec<Lisp>) -> Lisp {
    Lisp::List(Rc::new(items))
}
