// Clippy complains about A1Fns and A2Fns because
// they don't _have_ to be pass by value.
#![allow(needless_pass_by_value)]

use data::Address;
use data::Keyword;
use data::Literal;
use errors::*;
use std::collections::HashMap;
use std::fmt;
use std::usize;

pub mod list;
pub mod math;
pub mod util;

pub type StackFn = Fn(&mut Vec<Literal>) -> Result<()>;
pub type A1Fn = Fn(Literal) -> Result<Literal>;
pub type A2Fn = Fn(Literal, Literal) -> Result<Literal>;

pub enum Syscall {
    Stack(Box<StackFn>),
    A1(Box<A1Fn>),
    A2(Box<A2Fn>),
}

pub trait SyscallFactory {
    fn syscalls(&self) -> Vec<(Keyword, Syscall)>;
}

fn destatic(v: Vec<(&'static str, Syscall)>) -> Vec<(Keyword, Syscall)> {
    v.into_iter().map(|(k, s)| (k.to_string(), s)).collect()
}

#[derive(Default)]
pub struct SyscallRegistry {
    syscalls: HashMap<usize, Syscall>,
    idx: usize,
}

impl SyscallRegistry {
    pub fn new() -> SyscallRegistry {
        SyscallRegistry {
            syscalls: HashMap::new(),
            idx: 0,
        }
    }

    pub fn lookup(&self, addr: Address) -> Option<&Syscall> {
        let c = usize::MAX - addr.0;

        self.syscalls.get(&c)
    }

    pub fn contains(&self, addr: Address) -> bool {
        self.syscalls.contains_key(&(usize::MAX - addr.0))
    }

    pub fn cost(&self, _addr: Address) -> usize {
        20
    }

    pub fn ingest(&mut self, fact: &SyscallFactory) -> Vec<(String, Address)> {
        fact.syscalls()
            .into_iter()
            .map(|(name, syscall)| {
                self.syscalls.insert(self.idx, syscall);
                let a = (usize::MAX - self.idx, 0);
                self.idx += 1;
                (name, a)
            }).collect()
    }
}

impl fmt::Debug for SyscallRegistry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SyscallRegistry {{}}")
    }
}
