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
                Op::Lit(l) => {
                    self.stack.push(l);
                    ()
                }
                Op::Return => {
                    self.frames.pop().ok_or("Attempted to return on empty stack")?;
                    ()
                }
                Op::Call => {
                    let a = self
                        .stack
                        .pop()
                        .ok_or("Attempted to pop data stack for jump")?;
                    match a {
                        Literal::Address(a) => {
                            self.frames.push(a);
                            ()
                        }
                        _ => return Err("attempted to jump to non-address".into()),
                    };
                    ()
                }
                Op::Jump => {
                    let address = self
                        .stack
                        .pop()
                        .ok_or("Attempted to pop stack for address")?
                        .ensure_address()?;
                    self.jump(address)?;
                }
                Op::JumpCond => {
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
                        self.jump(then)?;
                    } else {
                        self.jump(els)?;
                    }
                }
                Op::PushEnv => {
                    self.environment.push();
                },
                Op::PopEnv => {
                    self.environment.pop()?;
                },
                Op::Load => {
                    let keyword = self.stack.pop()
                        .ok_or("Attempted to pop stack for keyword for load")?
                        .ensure_keyword()?;
                    let mut val = self.environment.get(&keyword)?;
                    let mut val = Rc::make_mut(&mut val);
                    self.stack.push(val.clone())
                },
                Op::Store => {
                    let keyword = self.stack.pop()
                        .ok_or("Attempted to pop stack for keyword for store")?
                        .ensure_keyword()?;
                    let value = self.stack.pop()
                        .ok_or("Attempted to pop stack for value for store")?;

                    self.environment.put(keyword, Rc::new(value));
                }
            };

            Ok(())
        }
    }
}
