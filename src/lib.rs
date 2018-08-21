// because clippy
#![allow(unknown_lints)]

#[macro_use]
extern crate failure;
extern crate im;
extern crate lalrpop_util;

pub mod ast;
pub mod builtin;
pub mod data;
mod environment;
pub mod errors;
pub mod interpreter;
pub mod parser;

// std::usize::MAX

pub mod vm {
    use std::fmt;
    use std::rc::Rc;

    use builtin;
    use data;
    use data::Address;
    use data::Literal;
    use environment::EnvStack;
    use errors::*;

    pub struct Bytecode {
        pub chunks: Vec<Chunk>,
    }

    #[derive(Debug)]
    pub struct Chunk {
        pub ops: Vec<Op>,
    }

    impl fmt::Debug for Bytecode {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "Bytecode {{compiled code}}")
        }
    }

    impl Bytecode {
        pub fn new(v: Vec<Vec<Op>>) -> Bytecode {
            Bytecode {
                chunks: v.into_iter().map(|c| Chunk { ops: c }).collect(),
            }
        }

        pub fn addr(&self, a: Address) -> Result<Op> {
            let chunk = self.chunks.get(a.0).ok_or(format_err!("Invalid chunk address: {:?}", a))?;
            let op = chunk.ops.get(a.1).ok_or(err_msg("Invalid operation address"))?;
            Ok(op.clone())
        }

        pub fn dissassemble(&self) {
            fn dissassemble_op(o: &Op) -> &'static str {
                match o {
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
                }
            }

            for (chunk_idx, chunk) in self.chunks.iter().enumerate() {
                println!("################ CHUNK #{:?} ################", chunk_idx);
                for (op_idx, op) in chunk.ops.iter().enumerate() {
                    let a = (chunk_idx, op_idx);

                    print!("\t{:?}\t{:}", a, dissassemble_op(&op));

                    if let Op::Lit(l) = op {
                        print!("\t{:?}", l);
                    }
                    println!()
                }
            }
        }
    }

    #[derive(Clone)]
    pub enum Op {
        Lit(data::Literal),
        Return,
        Call,
        Jump,
        // <else> <then> <pred>
        // If pred is true, jump to then, otherwise jump to else
        JumpCond,
        Load,
        Store,
        PushEnv,
        PopEnv,
        Dup,
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
            }
        }
    }

    #[derive(Debug)]
    pub struct VM {
        code: Bytecode,
        frames: Vec<data::Address>,
        stack: Vec<data::Literal>,
        builtin: builtin::Builtin,
        environment: EnvStack,
    }

    impl VM {
        pub fn new(code: Bytecode) -> VM {
            VM {
                code,
                frames: vec![(0, 0)],
                stack: vec![],
                builtin: builtin::Builtin::new(),
                environment: EnvStack::new(),
            }
        }

        fn pcounter(&mut self) -> Result<Address> {
            let pc = self.frames.last_mut().ok_or(err_msg("Stack empty, no counter"))?;
            let a = *pc;

            data::address_inc(pc);

            Ok(a)
        }

        pub fn step_until_value(&mut self, print: bool) -> Result<data::Literal> {
            loop {
                if self.frames.is_empty() {
                    return self
                        .stack
                        .pop()
                        .ok_or(err_msg("Frames empty, but no value to return"));
                }

                if print {
                    println!("{:?}", self);
                }

                self.single_step()?;
            }
        }

        pub fn jump(&mut self, addr: data::Address) -> Result<()> {
            let pc: &mut data::Address = self
                .frames
                .last_mut()
                .ok_or(err_msg("Frames empty, no way to jump"))?;

            *pc = addr;
            Ok(())
        }

        pub fn single_step(&mut self) -> Result<()> {
            let pc = self.pcounter()?;
            // TODO: maybe don't look up program chunk first?
            let op = match self.code.addr(pc) {
                Ok(x) => x,
                Err(e) => {
                    // TODO: This should only happen when chunk lookup fails
                    // Fix this when real error states are implemented.
                    if let Some(f) = self.builtin.lookup(pc) {
                        f(&mut self.stack).context(format_err!("while executing builtin at {:?}", pc))?;
                        self.frames.pop();
                        return Ok(());
                    }
                    // This is required because we can't return a context directly
                    Err(e).context("builtin lookup failed")?;
                    return Ok(()) // this never exeuctes
                }
            };

            // https://users.rust-lang.org/t/announcing-failure/13895/18
            Ok(match op {
                Op::Lit(l) => self.op_lit(l).context("Executing operation literal")?,
                Op::Return => self.op_return().context("Executing operation return")?,
                Op::Call => self.op_call().context("Executing operation call")?,
                Op::Jump => self.op_jump().context("Executing operation jump")?,
                Op::JumpCond => self
                    .op_jumpcond()
                    .context("Executing operation jumpcond")?,
                Op::Load => self.op_load().context("Executing operation load")?,
                Op::Store => self.op_store().context("Executing operation store")?,
                Op::PushEnv => self
                    .op_pushenv()
                    .context("Executing operation pushenv")?,
                Op::PopEnv => self.op_popenv().context("Executing operation popenv")?,
                Op::Dup => self.op_dup().context("Executing operation dup")?,
            })
        }

        fn op_lit(&mut self, l: data::Literal) -> Result<()> {
            self.stack.push(l);
            Ok(())
        }

        fn op_return(&mut self) -> Result<()> {
            self.frames
                .pop()
                .ok_or(err_msg("Attempted to return on empty stack"))?;
            Ok(())
        }

        fn op_call(&mut self) -> Result<()> {
            let a = self
                .stack
                .pop()
                .ok_or(err_msg("Attempted to pop data stack for jump"))?;

            if let Literal::Address(addr) = a {
                self.frames.push(addr);
                Ok(())
            } else {
                Err(err_msg("attempted to jump to non-address"))
            }
        }

        fn op_jump(&mut self) -> Result<()> {
            let address = self
                .stack
                .pop()
                .ok_or(err_msg("Attempted to pop stack for address"))?
                .ensure_address()?;

            self.jump(address)
        }

        fn op_jumpcond(&mut self) -> Result<()> {
            let cond = self
                .stack
                .pop()
                .ok_or(err_msg("Attempted to pop stack for conditional for if zero"))?
                .ensure_bool()?;

            let then = self
                .stack
                .pop()
                .ok_or(err_msg("Attepmted to pop stack for address for if true"))?
                .ensure_address()?;

            let els = self
                .stack
                .pop()
                .ok_or(err_msg("Attepmted to pop stack for address for if false"))?
                .ensure_address()?;

            if cond {
                self.jump(then)
            } else {
                self.jump(els)
            }
        }

        fn op_load(&mut self) -> Result<()> {
            let keyword = self
                .stack
                .pop()
                .ok_or(err_msg("Attempted to pop stack for keyword for load"))?
                .ensure_keyword()?;

            let mut val = self.environment.get(&keyword)?;

            let val = Rc::make_mut(&mut val);

            self.stack.push(val.clone());
            Ok(())
        }

        fn op_store(&mut self) -> Result<()> {
            let keyword = self
                .stack
                .pop()
                .ok_or(err_msg("Attempted to pop stack for keyword for store"))?
                .ensure_keyword()?;
            let value = self
                .stack
                .pop()
                .ok_or(err_msg("Attempted to pop stack for value for store"))?;

            self.environment.insert(keyword, Rc::new(value))?;

            Ok(())
        }
        fn op_pushenv(&mut self) -> Result<()> {
            self.environment.push();
            Ok(())
        }
        fn op_popenv(&mut self) -> Result<()> {
            self.environment.pop()?;
            Ok(())
        }
        fn op_dup(&mut self) -> Result<()> {
            let v = self
                .stack
                .last()
                .ok_or(err_msg("Attmempted to dup empty stack"))?
                .clone();
            self.stack.push(v);
            Ok(())
        }
    }
}
