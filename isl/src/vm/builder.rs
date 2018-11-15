use super::VMConfig;
use super::VMState;
use super::VM;
use data::Address;
use data::Keyword;
use data::Literal;
use env;
use errors::*;
use syscall;
use vm::bytecode::Bytecode;
use vm::bytecode::Chunk;
use vm::op::Op;
use vm::Frame;

/// Construct a VM.
#[derive(Default)]
pub struct Builder {
    codes: Vec<Bytecode>,
    sys_facts: Vec<Box<syscall::SyscallFactory>>,
    env: Vec<(Keyword, Literal)>,
    conf: VMConfig,
}

impl Builder {
    /// Create a new [`VM`] builder.
    pub fn new() -> Builder {
        Builder {
            codes: vec![],
            sys_facts: vec![],
            env: vec![],
            conf: Default::default(),
        }
    }

    /// Add the [`Bytecode`] to the [`VM`]. Multiple [`Bytecode`]s are called
    /// in the order they were added.
    pub fn code(&mut self, code: Bytecode) -> &mut Builder {
        self.codes.push(code);
        self
    }

    /// Added a [`syscall::SyscallFactory`] to the syscalls.
    pub fn syscalls(&mut self, fact: Box<syscall::SyscallFactory>) -> &mut Builder {
        self.sys_facts.push(fact);
        self
    }

    /// Add a series of default libraries to the [`VM`].
    ///
    /// Adds [`math`](syscall::math::Factory), [`list`](syscall::list::Factory),
    /// [`util`](syscall::util::Factory)
    pub fn default_libs(&mut self) -> &mut Builder {
        self.syscalls(Box::new(syscall::math::Factory::new()))
            .syscalls(Box::new(syscall::list::Factory::new()))
            .syscalls(Box::new(syscall::util::Factory::new()));

        self
    }

    /// Adds a key value pair to the global environment of the [`VM`].
    pub fn env(&mut self, k: Keyword, v: Literal) -> &mut Builder {
        self.env.push((k, v));
        self
    }

    // Config

    /// See [`VMConfig::reset_on_error`].
    pub fn reset_on_error(&mut self, reset: bool) -> &mut Self {
        self.conf.reset_on_error = reset;
        self
    }

    /// See [`VMConfig::print_trace`].
    pub fn print_trace(&mut self, print: bool) -> &mut Self {
        self.conf.print_trace = print;
        self
    }

    /// Consume the builder to construct the [`VM`] and return it.
    pub fn build(self) -> VM {
        // The first vec! is a dummy for the build_entry_chunk
        let mut code = Bytecode::new(vec![vec![]]);
        // Hold all the entry points.
        let mut entries = vec![];

        for c in self.codes {
            entries.push(code.import(&c));
        }

        code.chunks[0] = build_entry_chunk(&entries);

        let mut e = env::EnvStack::new();
        let mut sys = syscall::SyscallRegistry::new();

        // Put syscalls into the environment
        for f in self.sys_facts {
            syscall::ingest_environment(&mut sys, e.peek_mut().unwrap(), &*f);
        }

        // Then push the custom environment vars.
        for (k, v) in self.env {
            e.insert(k, v).unwrap();
        }

        VM {
            code,
            frames: vec![Frame::new((0, 0))],
            stack: vec![],
            sys,
            environment: e,
            state: VMState::Stopped,
            conf: self.conf,
            proc: None,
        }
    }

    /// Consume the builder to construct the [`VM`] and then execute to a value. Returns the value (or an error) an the VM.
    ///
    /// See [`VM::step_until_value`].
    pub fn build_exec(self) -> (Result<Literal>, VM) {
        let mut vm = self.build();

        vm.code.dissassemble();

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

    if entries.is_empty() {
        ops.push(Op::Lit(false.into()));
    }

    ops.push(Op::Return);

    Chunk { ops }
}
