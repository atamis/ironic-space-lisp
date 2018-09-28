//! Single VM executable operations.

use data;
use std::fmt;

/// Basic operations (or instructions).
///
/// Manually implements `Debug` to provide short 2-3 character names.
#[derive(Clone, PartialEq)]
pub enum Op {
    /// Pushes a literal datum to the stack.
    Lit(data::Literal),

    /// Pop the frame stack to return from a function.
    ///
    /// Note that returning from the top level function terminates the VM and provides an ultimate return value.
    Return,

    /// Push an address to the frame stack to call a function.
    ///
    /// `<addr>`
    Call,

    /// Unconditional jump to an address
    Jump,
    /// Conditionally jump to one of two addresses. This is pretty inconvenient to use by hand.
    /// If pred is true, jump to then, otherwise jump to else
    ///
    /// `<else then pred>`
    ///
    /// Where else and then are addresses and pred is a boolean.
    JumpCond,

    /// Load a value from the environment
    ///
    /// `<keyword>`
    Load,

    /// Store a value from the stack in the environment.
    ///
    /// `<value keyword>`
    Store,

    /// Push an Environment onto the environment stack (see the `environment` module).
    PushEnv,

    /// Pop an environment from the stack.
    PopEnv,

    /// Duplicates the top item of the stack.
    ///
    /// `<item>`
    Dup,

    /// Pop an item from the stack.
    ///
    /// `<item>`
    Pop,

    /// Make a closure from an address and an arity
    ///
    /// `<address arity>`
    MakeClosure,

    /// Call a function with a given arity
    ///
    /// parameter: arity
    CallArity(usize),

    /// Wait for an external message.
    ///
    /// Puts the next message recieved onto the stack.
    Wait,
}

impl Op {
    /// A nice human readable name for the `Bytecode::dissassemble` method.
    pub fn dissassemble(&self) -> &'static str {
        match self {
            Op::Lit(_) => "Lit",
            Op::Return => "Return",
            Op::Call => "Call",
            Op::Jump => "Jump",
            Op::JumpCond => "JumpCond",
            Op::Load => "Load",
            Op::Store => "Store",
            Op::PushEnv => "PushEnv",
            Op::PopEnv => "PopEnv",
            Op::Dup => "Dup",
            Op::Pop => "Pop",
            Op::MakeClosure => "MkClosure",
            Op::CallArity(_) => "CallArity",
            Op::Wait => "Wait",
        }
    }

    /// The "cost" of executing an operation in terms of some abstract resource.
    pub fn cost(&self) -> usize {
        10
    }
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Lit(l) => write!(f, "l({:?})", l),
            Op::Return => write!(f, "oR"),
            Op::Call => write!(f, "oC"),
            Op::Jump => write!(f, "oJ"),
            Op::JumpCond => write!(f, "oJ?"),
            Op::Load => write!(f, "oL"),
            Op::Store => write!(f, "oS"),
            Op::PushEnv => write!(f, "oPuE"),
            Op::PopEnv => write!(f, "oPoE"),
            Op::Dup => write!(f, "oD"),
            Op::Pop => write!(f, "oP"),
            Op::MakeClosure => write!(f, "oMkC"),
            Op::CallArity(a) => write!(f, "oC{:}", a),
            Op::Wait => write!(f, "oW"),
        }
    }
}
