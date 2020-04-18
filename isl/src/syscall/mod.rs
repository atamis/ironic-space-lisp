//! Utilities for registering, managing, and invoking syscalls.
//!
//! This is heavily integrated with the [`VM`](crate::vm::VM), and they should be read together.

// Clippy complains about A1Fns and A2Fns because
// they don't _have_ to be pass by value.
#![allow(clippy::needless_pass_by_value)]

use crate::data::Address;
use crate::data::Literal;
use crate::data::Symbol;
use crate::env;
use crate::errors::*;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::usize;

pub mod list;
pub mod math;
pub mod util;

/// A syscall that mutates a stack directly.
pub type StackFn = Box<dyn Fn(&mut Vec<Literal>) -> Result<()> + Send + Sync + 'static>;

/// A syscall that takes 1 value and returns 1 value.
pub type A1Fn = Box<dyn Fn(Literal) -> Result<Literal> + Send + Sync + 'static>;

/// A syscall that takes 2 values and returns 1 value.
pub type A2Fn = Box<dyn Fn(Literal, Literal) -> Result<Literal> + Sync + Send + 'static>;

/// A syscall that takes 3 values and returns 1 value.
pub type A3Fn = Box<dyn Fn(Literal, Literal, Literal) -> Result<Literal> + Sync + Send + 'static>;

/// Tagged pointers to syscall implementations.
pub enum Syscall {
    /// A stack function.
    Stack(StackFn),
    /// Arity-1 function.
    A1(A1Fn),
    /// Arity-2 function.
    A2(A2Fn),

    /// Arity-3 function
    A3(A3Fn),
}

impl Syscall {
    /// The arity of the syscall, or None if it's a [`StackFn`], whose arity can't be determined.
    pub fn arity(&self) -> Option<usize> {
        match self {
            Syscall::Stack(_) => None,
            Syscall::A1(_) => Some(1),
            Syscall::A2(_) => Some(2),
            Syscall::A3(_) => Some(3),
        }
    }
}

/// Produces a list of names and syscalls.
pub trait SyscallFactory {
    /// Returns a list associating a name with a syscall function pointer.
    fn syscalls(&self) -> Vec<(Symbol, Syscall)>;
}

/// Convert static strings to String structs. Useful for naming syscalls after string literals.
fn destatic(v: Vec<(&'static str, Syscall)>) -> Vec<(Symbol, Syscall)> {
    v.into_iter().map(|(k, s)| (k.to_string(), s)).collect()
}

/// Keeps track of installed syscalls and their pseudo-[`Address`]
#[derive(Default, Clone)]
pub struct SyscallRegistry {
    syscalls: HashMap<usize, Arc<Syscall>>,
    idx: usize,
}

impl SyscallRegistry {
    /// Create a new empty [`SyscallRegistry`].
    pub fn new() -> SyscallRegistry {
        SyscallRegistry {
            syscalls: HashMap::new(),
            idx: 0,
        }
    }

    /// Look up a syscall, returning `None` if not found.
    pub fn lookup(&self, addr: Address) -> Option<&Syscall> {
        let c = usize::MAX - addr.0;

        self.syscalls.get(&c).map(|v| &**v)
    }

    /// Is this address a valid syscall address.
    pub fn contains(&self, addr: Address) -> bool {
        self.syscalls.contains_key(&(usize::MAX - addr.0))
    }

    /// The cost of executing this syscall. See [`cost()`](super::vm::op::Op::cost()) for more information.
    pub fn cost(&self, _addr: Address) -> usize {
        20
    }

    /// Insert the syscalls from a [`SyscallFactory`] into this registry, returning a `Vec` of
    /// `(name, arity?, Address)`.
    ///
    /// This is intended to be used to associated the name with the address in some runtime name binding,
    /// possiblly with the arity in a [`Closure`](super::data::Literal::Closure).
    pub fn ingest(&mut self, fact: &dyn SyscallFactory) -> Vec<(String, Option<usize>, Address)> {
        fact.syscalls()
            .into_iter()
            .map(|(name, syscall)| {
                let arity = syscall.arity();
                self.syscalls.insert(self.idx, Arc::new(syscall));
                let a = (usize::MAX - self.idx, 0);
                self.idx += 1;
                (name, arity, a)
            })
            .collect()
    }
}

impl fmt::Debug for SyscallRegistry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SyscallRegistry {{}}")
    }
}

/// Use a [`SyscallFactory`], registering the syscalls with the [`SyscallRegistry`],
/// and the names with the [`env::Env`].
pub fn ingest_environment(
    sys: &mut SyscallRegistry,
    env: &mut env::Env,
    fact: &dyn SyscallFactory,
) {
    for (name, arity_opt, addr) in sys.ingest(fact) {
        let f = match arity_opt {
            Some(n) => Literal::Closure(n, addr),
            None => Literal::Address(addr),
        };

        env.insert(name, f);
    }
}
