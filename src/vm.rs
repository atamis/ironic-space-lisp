//! Bytecode definition and VM for bytecode execution.

use std::fmt;
use std::rc::Rc;

use builtin;
use data;
use data::Address;
use data::Literal;
use environment::EnvStack;
use errors::*;

/// Holds `Chunk`s of bytecode. See `Bytecode::addr` for its primary use.
#[derive(Clone)]
pub struct Bytecode {
    pub chunks: Vec<Chunk>,
}


/// A `Vec` of operations
#[derive(Debug, Clone)]
pub struct Chunk {
    pub ops: Vec<Op>,
}

impl Chunk {
    pub fn dissassemble(&self, chunk_idx: usize) {
        for (op_idx, op) in self.ops.iter().enumerate() {
            let a = (chunk_idx, op_idx);

            print!("\t{:?}\t{:}", a, op.dissassemble());

            if let Op::Lit(l) = op {
                print!("\t{:?}", l);
            }
            println!()
        }
    }
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

    /// Indexes into the chunks to find the indicated operation.
    pub fn addr(&self, a: Address) -> Result<Op> {
        let chunk = self
            .chunks
            .get(a.0)
            .ok_or(format_err!("Invalid chunk address: {:?}", a))?;
        let op = chunk
            .ops
            .get(a.1)
            .ok_or(err_msg("Invalid operation address"))?;
        Ok(op.clone())
    }

    /// Prints a plain text disassembly of all the chunks to STDOUT.
    pub fn dissassemble(&self) {
        for (chunk_idx, chunk) in self.chunks.iter().enumerate() {
            println!("################ CHUNK #{:?} ################", chunk_idx);
            chunk.dissassemble(chunk_idx);
        }
    }
}

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
}

impl Op {
    /// A nice human readable name for the `Bytecode::dissassemble` method.
    fn dissassemble(&self) -> &'static str {
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
        }
    }

    /// The "cost" of executing an operation in terms of some abstract resource.
    fn cost(&self) -> usize {
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
        }
    }
}

/// A non-reusable bytecode VM.
///
/// Keeps track of data stack, frame stack, environment stack, and the code.
#[derive(Debug)]
pub struct VM {
    code: Bytecode,
    frames: Vec<data::Address>,
    stack: Vec<data::Literal>,
    builtin: builtin::Builtin,
    environment: EnvStack,
}

impl VM {
    /// Create a VM loaded with the provided code. Program counter is initially `(0, 0)`.
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
        let pc = self
            .frames
            .last_mut()
            .ok_or(err_msg("Stack empty, no counter"))?;
        let a = *pc;

        data::address_inc(pc);

        Ok(a)
    }

    fn pc_peek(&self) -> Result<Op> {
        let pc = self
            .frames
            .last()
            .ok_or(err_msg("Stack empty, no counter"))?;

        self.code.addr(*pc)
    }

    /// Step until a "top-level" return, which is when the frame stack is empty.
    /// At this point, the stack is popped and returned. A failure to pop a value
    /// is treated as an error state. Propagates errors from `single_step`. If
    /// `print` is `true`, print the VM state on every state.
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

    /// Step until a resource is consumed. Each operation executed decrements a counter
    /// initially set to `max`. As with `step_until_value`, the lack of a return value
    /// is treated as an error.
    ///
    /// Returns `Err` if an error is encountered
    ///
    /// `Ok(None)` if the resource pool was exhausted
    ///
    /// `Ok(Some(_))` if there was a top level return.
    pub fn step_until_cost(&mut self, max: usize) -> Result<Option<data::Literal>> {
        let mut c = max;
        loop {
            // peek next op
            let op = self
                .pc_peek()
                .context("Peeking pc while executing until cost")?;

            // check cost
            if c < op.cost() {
                return Ok(None);
            }

            // execute single step
            self.single_step()
                .context("Executing until cost exceeds max")?;

            // check return
            if self.frames.is_empty() {
                return Ok(Some(
                    self.stack
                        .pop()
                        .ok_or(err_msg("Frames empty, but no value to return"))?,
                ));
            }

            // decr cost
            c -= op.cost()
        }
    }

    /// Manually jump the VM to an address. This returns an `Err` if the frame
    /// stack is empty.
    pub fn jump(&mut self, addr: data::Address) -> Result<()> {
        let pc: &mut data::Address = self
            .frames
            .last_mut()
            .ok_or(err_msg("Frames empty, no way to jump"))?;

        *pc = addr;
        Ok(())
    }

    /// Loads new code into the VM, and resets the data and frame stack.
    pub fn reset(&mut self, code: Bytecode) {
        self.code = code;
        self.stack = vec![];
        self.frames = vec![(0, 0)];
    }

    /// Execute a single operation. Returns an `Err` if an error was encountered,
    /// or `Ok(())` if it was successful. No particular attempt has been made to make
    /// `Err`s survivable, but no particular attempt has been made to prevent further
    /// execution. No attempt has been made to attempt to maintain operation arity in
    /// error states. See `fn op_*` for raw implementations, an the documentation for `Op`
    /// for high level descriptions of the operations.
    pub fn single_step(&mut self) -> Result<()> {
        let pc = self.pcounter()?;
        // TODO: maybe don't look up program chunk first?
        let op = match self.code.addr(pc) {
            Ok(x) => x,
            Err(e) => {
                // TODO: This should only happen when chunk lookup fails
                // Fix this when real error states are implemented.
                if let Some(f) = self.builtin.lookup(pc) {
                    f(&mut self.stack)
                        .context(format_err!("while executing builtin at {:?}", pc))?;
                    self.frames.pop();
                    return Ok(());
                }
                // This is required because we can't return a context directly
                Err(e).context("builtin lookup failed")?;
                return Ok(()); // this never exeuctes
            }
        };

        self.exec_op(op)
    }

    /// Execute a single operation, ignoring any already loaded code and ignoring the
    /// program counter. See `single_step` for more details.
    pub fn exec_op(&mut self, op: Op) -> Result<()> {
        // https://users.rust-lang.org/t/announcing-failure/13895/18
        Ok(match op {
            Op::Lit(l) => self.op_lit(l).context("Executing operation literal")?,
            Op::Return => self.op_return().context("Executing operation return")?,
            Op::Call => self.op_call().context("Executing operation call")?,
            Op::Jump => self.op_jump().context("Executing operation jump")?,
            Op::JumpCond => self.op_jumpcond().context("Executing operation jumpcond")?,
            Op::Load => self.op_load().context("Executing operation load")?,
            Op::Store => self.op_store().context("Executing operation store")?,
            Op::PushEnv => self.op_pushenv().context("Executing operation pushenv")?,
            Op::PopEnv => self.op_popenv().context("Executing operation popenv")?,
            Op::Dup => self.op_dup().context("Executing operation dup")?,
            Op::Pop => self.op_pop().context("Executing operation pop")?,
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

    // Currently, this doesn't always consume 3 stack items.
    // This may need to change.
    fn op_jumpcond(&mut self) -> Result<()> {
        let cond = self
            .stack
            .pop()
            .ok_or(err_msg(
                "Attempted to pop stack for conditional for if zero",
            ))?;

        let then = self
            .stack
            .pop()
            .ok_or(err_msg("Attempted to pop stack for address for if true"))?
            .ensure_address()?;

        let els = self
            .stack
            .pop()
            .ok_or(err_msg("Attempted to pop stack for address for if false"))?
            .ensure_address()?;

        if cond.truthy() {
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

    fn op_pop(&mut self) -> Result<()> {
        self.stack.pop().ok_or(err_msg("Attempted to pop empty stack"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_bytecode_errors() {
        let empty = Bytecode::new(vec![]);
        assert!(empty.addr((0, 0)).is_err());

        let single = Bytecode::new(vec![vec![Op::Return]]);
        let maybe_ret = single.addr((0, 0));
        assert!(maybe_ret.is_ok());
        assert_eq!(maybe_ret.unwrap(), Op::Return);
        assert!(single.addr((0, 1)).is_err());
        assert!(single.addr((1, 0)).is_err());
    }

    #[test]
    fn test_pcounter() {
        let single = Bytecode::new(vec![vec![Op::Return]]);

        let mut vm = VM::new(single);

        let a = vm.pcounter();
        assert!(a.is_ok());
        assert_eq!(a.unwrap(), (0, 0));

        let b = vm.pcounter();
        assert!(b.is_ok());
        assert_eq!(b.unwrap(), (0, 1));

        vm.frames.pop().unwrap();

        assert!(vm.pcounter().is_err());
    }

    #[test]
    fn test_jump() {
        let single = Bytecode::new(vec![vec![Op::Return]]);
        let mut vm = VM::new(single);

        vm.jump((5, 5)).unwrap();
        assert_eq!(*vm.frames.last().unwrap(), (5, 5));
    }

    #[test]
    fn test_op_lit() {
        let empty = Bytecode::new(vec![vec![]]);
        let mut vm = VM::new(empty);

        vm.op_lit(Literal::Number(0)).unwrap();
        assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0))
    }

    #[test]
    fn test_op_return() {
        let empty = Bytecode::new(vec![vec![]]);
        let mut vm = VM::new(empty);

        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_return().unwrap();
        assert!(vm.frames.is_empty());
    }

    #[test]
    fn test_op_call() {
        let empty = Bytecode::new(vec![vec![]]);
        let mut vm = VM::new(empty);

        vm.op_lit(Literal::Number(0)).unwrap();
        assert!(vm.op_call().is_err());
        assert!(vm.stack.is_empty()); // only going to test this once

        vm.op_lit(Literal::Address((0, 0))).unwrap();
        assert!(vm.op_call().is_ok());
        assert_eq!(*vm.frames.last().unwrap(), (0, 0));
        assert_eq!(vm.frames.len(), 2)
    }

    #[test]
    fn test_op_jump() {
        let empty = Bytecode::new(vec![vec![]]);
        let mut vm = VM::new(empty);

        vm.op_lit(Literal::Number(0)).unwrap();
        assert!(vm.op_jump().is_err());

        vm.op_lit(Literal::Address((5, 5))).unwrap();
        assert!(vm.op_jump().is_ok());
        assert_eq!(*vm.frames.last().unwrap(), (5, 5));
    }

    #[test]
    fn test_jumpcond_then() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        vm.op_lit(Literal::Address((6, 0))).unwrap();
        vm.op_lit(Literal::Address((5, 0))).unwrap();
        vm.op_lit(Literal::Boolean(true)).unwrap();
        assert!(vm.op_jumpcond().is_ok());
        assert_eq!(*vm.frames.last().unwrap(), (5, 0));
    }

    #[test]
    fn test_jumpcond_else() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        vm.op_lit(Literal::Address((6, 0))).unwrap();
        vm.op_lit(Literal::Address((5, 0))).unwrap();
        vm.op_lit(Literal::Boolean(false)).unwrap();
        assert!(vm.op_jumpcond().is_ok());
        assert_eq!(*vm.frames.last().unwrap(), (6, 0));
    }

    #[test]
    fn test_jumpcond_errors() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_lit(Literal::Address((5, 0))).unwrap();
        vm.op_lit(Literal::Boolean(false)).unwrap();
        assert!(vm.op_jumpcond().is_err());

        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        vm.op_lit(Literal::Address((6, 0))).unwrap();
        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_lit(Literal::Boolean(false)).unwrap();
        assert!(vm.op_jumpcond().is_err());


        // Now uses Literal::truthy, which is defined for all values.
        /*let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        vm.op_lit(Literal::Address((6, 0))).unwrap();
        vm.op_lit(Literal::Address((5, 0))).unwrap();
        vm.op_lit(Literal::Number(1)).unwrap();
        assert!(vm.op_jumpcond().is_err());*/
    }

    #[test]
    fn test_op_load() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        assert!(vm.environment.get("test").is_err());
        vm.environment
            .insert("test".to_string(), Rc::new(Literal::Number(0)))
            .unwrap();
        assert_eq!(*vm.environment.get("test").unwrap(), Literal::Number(0));
        vm.op_lit(Literal::Keyword("test".to_string())).unwrap();
        vm.op_load().unwrap();
        assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));
    }

    #[test]
    fn test_op_store() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        assert!(vm.environment.get("test").is_err());
        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_lit(Literal::Keyword("test".to_string())).unwrap();
        vm.op_store().unwrap();
        assert_eq!(*vm.environment.get("test").unwrap(), Literal::Number(0));
    }

    #[test]
    fn test_op_pushenv_popenv() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));

        vm.environment
            .insert("test1".to_string(), Rc::new(Literal::Number(0)))
            .unwrap();
        assert!(vm.environment.get("test2").is_err());

        vm.op_pushenv().unwrap();

        assert_eq!(*vm.environment.get("test1").unwrap(), Literal::Number(0));

        vm.environment
            .insert("test2".to_string(), Rc::new(Literal::Number(1)))
            .unwrap();
        assert_eq!(*vm.environment.get("test2").unwrap(), Literal::Number(1));
        vm.op_lit(Literal::Keyword("test1".to_string())).unwrap();
        vm.op_load().unwrap();
        assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));

        vm.op_popenv().unwrap();
        assert_eq!(*vm.environment.get("test1").unwrap(), Literal::Number(0));
        assert!(vm.environment.get("test2").is_err());
    }

    #[test]
    fn test_op_dup() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));
        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_dup().unwrap();

        assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));
        vm.stack.pop().unwrap();
        assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));

        vm.stack.pop().unwrap(); // empty the stack

        assert!(vm.op_dup().is_err());
    }

    #[test]
    fn test_op_pop() {
        let mut vm = VM::new(Bytecode::new(vec![vec![]]));
        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_pop().unwrap();

        assert_eq!(vm.stack.len(), 0);

        assert!(vm.op_pop().is_err());
    }

    #[test]
    fn test_step_until() {
        let mut ret = VM::new(Bytecode::new(vec![vec![Op::Return]]));
        assert!(ret.step_until_value(false).is_err());

        let mut ret = VM::new(Bytecode::new(vec![vec![
            Op::Lit(Literal::Number(0)),
            Op::Return,
        ]]));

        assert_eq!(ret.step_until_value(false).unwrap(), Literal::Number(0));

        // lol
        /*let mut never = VM::new(Bytecode::new(vec![vec![Op::Lit(Literal::Address((0, 0))),
                                                      Op::Jump,
                                                      Op::Return]]));
        assert_never_terminates!(never.step_until_value(false));*/

        let mut empty = VM::new(Bytecode::new(vec![vec![]]));
        assert!(ret.step_until_value(false).is_err());
        assert!(empty.single_step().is_err());
    }

    #[test]
    fn test_step_until_cost() {
        let mut ret = VM::new(Bytecode::new(vec![vec![
            Op::Lit(Literal::Number(0)),
            Op::Return,
        ]]));

        let res = ret.step_until_cost(0);
        println!("{:?}", res);

        assert!(res.is_ok());
        assert!(res.unwrap().is_none());

        let res = ret.step_until_cost(50);

        assert!(res.is_ok());
        assert_eq!(res.unwrap().unwrap(), Literal::Number(0));

        let res = ret.step_until_cost(50);

        assert!(res.is_err());

        let mut ret = VM::new(Bytecode::new(vec![vec![
            Op::Lit(Literal::Number(0)),
            Op::Return,
        ]]));

        // Partial
        let res = ret.step_until_cost(7);

        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[bench]
    fn bench_nested_envs(b: &mut Bencher) {
        use ast::AST;
        use compiler::compile;
        use compiler::pack_start;
        use str_to_ast;

        let s = "(let (x 0) (let (y 1) (let (z 2) x)))";
        let asts = str_to_ast(s).unwrap();
        let ast = AST::Do(asts);

        let ir = compile(&ast).unwrap();

        let code = pack_start(&ir).unwrap();

        code.dissassemble();

        let mut vm = VM::new(code);

        b.iter(|| {
            vm.frames.push((0, 0));
            vm.step_until_cost(10000).unwrap().unwrap();
        } )
    }
}
