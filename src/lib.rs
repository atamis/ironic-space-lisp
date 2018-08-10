#[macro_use]
extern crate error_chain;

mod builtin;
pub mod data;
pub mod errors;

// std::usize::MAX

pub mod vm {
    use std::fmt;

    use builtin;
    use data;
    use data::Address;
    use data::Literal;
    use errors::*;

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
        // predicate address IfZ
        // If predicate is 0, jump
        // can't really return directly from this sub-chunk
        // Implement frame-pop operation?
        IfZ,
    }

    impl fmt::Debug for Op {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                Op::Lit(l) => write!(f, "l({:?})", l),
                Op::Return => write!(f, "oR"),
                Op::Call => write!(f, "oC"),
                Op::IfZ => write!(f, "oIfZ"),
            }
        }
    }

    #[derive(Debug)]
    pub struct VM {
        code: Bytecode,
        frames: Vec<data::Address>,
        stack: Vec<data::Literal>,
        builtin: builtin::Builtin,
    }

    impl VM {
        pub fn new(code: Bytecode) -> VM {
            VM {
                code: code,
                frames: vec![(0, 0)],
                stack: vec![],
                builtin: builtin::Builtin::new(),
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
                    self.frames.pop();
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
                Op::IfZ => {
                    let address = self
                        .stack
                        .pop()
                        .ok_or("Attepmted to pop stack for address for if zero")?
                        .ensure_address()?;
                    let cond = self
                        .stack
                        .pop()
                        .ok_or("Attempted to pop stack for conditional for if zero")?
                        .ensure_number()?;

                    if cond == 0 {
                        self.frames.push(address);
                    }
                }
            };

            Ok(())
        }
    }
}
