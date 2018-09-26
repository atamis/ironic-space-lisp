use super::VMConfig;
use super::VMState;
use super::VM;
use data::Address;
use data::Keyword;
use data::Literal;
use env;
use errors::*;
use std::rc::Rc;
use syscall;
use vm::bytecode::Bytecode;
use vm::bytecode::Chunk;
use vm::op::Op;

/// Construct a VM.
#[derive(Default)]
pub struct Builder {
    codes: Vec<Bytecode>,
    sys_facts: Vec<Box<syscall::SyscallFactory>>,
    env: Vec<(Keyword, Literal)>,
    conf: VMConfig,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            codes: vec![],
            sys_facts: vec![],
            env: vec![],
            conf: Default::default(),
        }
    }

    pub fn code(&mut self, code: Bytecode) -> &mut Builder {
        self.codes.push(code);
        self
    }

    pub fn syscalls(&mut self, fact: Box<syscall::SyscallFactory>) -> &mut Builder {
        self.sys_facts.push(fact);
        self
    }

    pub fn default_libs(&mut self) -> &mut Builder {
        self.syscalls(Box::new(syscall::math::Factory::new()));
        self.syscalls(Box::new(syscall::list::Factory::new()));
        self.syscalls(Box::new(syscall::util::Factory::new()));

        self
    }

    pub fn env(&mut self, k: Keyword, v: Literal) -> &mut Builder {
        self.env.push((k, v));
        self
    }

    // Config

    pub fn reset_on_error(&mut self, reset: bool) -> &mut Self {
        self.conf.reset_on_error = reset;
        self
    }

    pub fn print_trace(&mut self, print: bool) -> &mut Self {
        self.conf.print_trace = print;
        self
    }

    pub fn build(self) -> VM {
        let mut code = Bytecode::new(vec![vec![]]);
        let mut entries = vec![];

        for c in self.codes {
            entries.push(code.import(&c));
        }

        code.chunks[0] = build_entry_chunk(&entries);

        let mut e = env::EnvStack::new();
        let mut sys = syscall::SyscallRegistry::new();

        for f in self.sys_facts {
            syscall::ingest_environment(&mut sys, e.peek_mut().unwrap(), &*f);
        }

        for (k, v) in self.env {
            e.insert(k, v).unwrap();
        }

        VM {
            code,
            frames: vec![(0, 0)],
            stack: vec![],
            sys,
            environment: e,
            state: VMState::Stopped,
            conf: self.conf,
        }
    }

    pub fn build_exec(self) -> (Result<Literal>, VM) {
        let mut vm = self.build();
        let res = vm.step_until_value();

        (res, vm)
    }
}

fn build_entry_chunk(entries: &[Address]) -> Chunk {
    let mut ops = Vec::with_capacity(entries.len() * 3 + 1);

    for (idx, a) in entries.iter().enumerate() {
        ops.append(&mut vec![Op::Lit(Literal::Address(*a)), Op::Call]);
        if idx < entries.len() - 1 {
            ops.push(Op::Pop);
        }
    }

    ops.push(Op::Return);

    Chunk { ops }
}
