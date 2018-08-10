#[macro_use]
extern crate error_chain;

pub mod builtin;
pub mod data;
pub mod errors;
mod environment;

// std::usize::MAX

pub mod vm {
    use std::fmt;
    use std::rc::Rc;

    use builtin;
    use data;
    use data::Address;
    use data::Literal;
    use errors::*;
    use environment::Environment;

    #[derive(Debug)]
    pub struct Bytecode {
        pub chunks: Vec<Chunk>,
    }

    #[derive(Debug)]
    pub struct Chunk {
        pub ops: Vec<Op>,
    }

    impl Bytecode {
        pub fn addr(&self, a: Address) -> Result<Op> {
            let chunk = self.chunks.get(a.0).ok_or("Invalid chunk address")?;
            let op = chunk.ops.get(a.1).ok_or("Invalid operation address")?;
            Ok(op.clone())
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
        environment: Environment,
    }

    impl VM {
        pub fn new(code: Bytecode) -> VM {
            VM {
                code: code,
                frames: vec![(0, 0)],
                stack: vec![],
                builtin: builtin::Builtin::new(),
                environment: Environment::new(),
            }
        }

        fn pcounter(&mut self) -> Result<Address> {
            let pc = self.frames.last_mut().ok_or("Stack empty, no counter")?;
            let a = pc.clone();

            data::address_inc(pc);

            Ok(a)
        }

        pub fn step_until_value(&mut self, print: bool) -> Result<data::Literal> {
            loop {
                if self.frames.len() == 0 {
                    return self
                        .stack
                        .pop()
                        .ok_or("Frames empty, but no value to return".into());
                }

                if print {
                    println!("{:?}", self);
                }

                self.single_step()?;
            }
        }

        pub fn jump(&mut self, addr: data::Address) -> Result<()> {
            let pc:&mut data::Address = self
                .frames.last_mut()
                .ok_or("Frames empty, no way to jump")?;

            *pc = addr;
            return Ok(());
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
                        f(&mut self.stack)?;
                        self.frames.pop();
                        return Ok(());
                    }
                    return Err(e).chain_err(|| "builtin lookup failed");
                }
            };

            match op {
                Op::Lit(l) => self.op_lit(l).chain_err(|| "Executing operation literal"),
                Op::Return => self.op_return().chain_err(|| "Executing operation return"),
                Op::Call => self.op_call().chain_err(|| "Executing operation call"),
                Op::Jump => self.op_jump().chain_err(|| "Executing operation jump"),
                Op::JumpCond => self.op_jumpcond().chain_err(|| "Executing operation jumpcond"),
                Op::Load => self.op_load().chain_err(|| "Executing operation load"),
                Op::Store => self.op_store().chain_err(|| "Executing operation store"),
                Op::PushEnv => self.op_pushenv().chain_err(|| "Executing operation pushenv"),
                Op::PopEnv => self.op_popenv().chain_err(|| "Executing operation popenv"),
                Op::Dup => self.op_dup().chain_err(|| "Executing operation dup"),
            }
        }

        fn op_lit(&mut self, l: data::Literal) -> Result<()> {
            self.stack.push(l);
            Ok(())
        }

        fn op_return(&mut self) -> Result<()> {
            self.frames.pop().ok_or("Attempted to return on empty stack")?;
            Ok(())
        }

        fn op_call(&mut self) -> Result<()> {
            let a = self
                .stack
                .pop()
                .ok_or("Attempted to pop data stack for jump")?;

            if let Literal::Address(addr) = a {
                self.frames.push(addr);
                Ok(())
            } else {
                Err("attempted to jump to non-address".into())
            }
        }

        fn op_jump(&mut self) -> Result<()> {
            let address = self
                .stack
                .pop()
                .ok_or("Attempted to pop stack for address")?
                .ensure_address()?;

            self.jump(address)
        }

        fn op_jumpcond(&mut self) -> Result<()> {
            let cond = self
                .stack
                .pop()
                .ok_or("Attempted to pop stack for conditional for if zero")?
                .ensure_bool()?;

            let then = self
                .stack
                .pop()
                .ok_or("Attepmted to pop stack for address for if true")?
                .ensure_address()?;

            let els = self
                .stack
                .pop()
                .ok_or("Attepmted to pop stack for address for if false")?
                .ensure_address()?;

            if cond {
                self.jump(then)
            } else {
                self.jump(els)
            }
        }

        fn op_load(&mut self) -> Result<()> {
            let keyword = self.stack.pop()
                .ok_or("Attempted to pop stack for keyword for load")?
                .ensure_keyword()?;

            let mut val = self.environment.get(&keyword)?;

            let mut val = Rc::make_mut(&mut val);

            self.stack.push(val.clone());
            Ok(())
        }

        fn op_store(&mut self) -> Result<()> {
            let keyword = self.stack.pop()
                .ok_or("Attempted to pop stack for keyword for store")?
                .ensure_keyword()?;
            let value = self.stack.pop()
                .ok_or("Attempted to pop stack for value for store")?;

            self.environment.put(keyword, Rc::new(value));

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
            let v = self.stack.last().ok_or("Attmempted to dup empty stack")?.clone();
            self.stack.push(v);
            Ok(())
        }
    }
}
