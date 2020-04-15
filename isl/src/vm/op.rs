//! Single VM executable operations.

use crate::data;
use std::fmt;

/// Basic operations (or instructions).
///
/// Manually implements `Debug` to provide short 2-3 character names.
/// Arguments are provided in the order they're popped off the stack.
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
    /// `<pred then else>`
    ///
    /// Where else and then are addresses and pred is a boolean.
    JumpCond,

    /// Load a value from the environment
    ///
    /// `<keyword>`
    Load,

    /// Store a value from the stack in the environment.
    ///
    /// `<keyword value>`
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
    /// `<arity address>`
    MakeClosure,

    /// Call a function with a given arity
    ///
    /// parameter: arity
    CallArity(usize),

    /// Wait for an external message.
    ///
    /// Puts the next message recieved onto the stack.
    Wait,

    /// Send an external message. Returns the pid.
    ///
    /// `<pid data>`
    Send,

    /// Returns the Pid of this VM, if available.
    ///
    /// Puts this VM's Pid on the stack, or #f if the VM has no Pid.
    Pid,

    /// Fork this VM, returning #t if in the forked VM, #f if in the orignal.
    ///
    /// Throws an error if this VM does not have an execution handle installed.
    Fork,

    /// Watch the `pid`, receiving the message `[:exit <pid>]` when it exits.
    ///
    /// `<pid>`
    Watch,

    /// Load a local var.
    ///
    /// parameter: index
    LoadLocal(usize),

    /// Store to a local var.
    ///
    /// parameter: index
    /// `<value>`
    StoreLocal(usize),

    /// Terminate the VM immediately, returning the value. This should also
    /// empty the frames and stack.
    ///
    /// `<value>`
    Terminate,
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
            Op::Send => "Send",
            Op::Fork => "Fork",
            Op::Pid => "Pid",
            Op::Watch => "Watch",
            Op::LoadLocal(_) => "LoadLocal",
            Op::StoreLocal(_) => "StoreLocal",
            Op::Terminate => "Terminate",
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
            Op::Wait => write!(f, "o<"),
            Op::Send => write!(f, "o>"),
            Op::Fork => write!(f, "oF"),
            Op::Pid => write!(f, "oMe"),
            Op::Watch => write!(f, "oW"),
            Op::LoadLocal(i) => write!(f, "oLL{:}", i),
            Op::StoreLocal(i) => write!(f, "oSL{:}", i),
            Op::Terminate => write!(f, "oT"),
        }
    }
}
