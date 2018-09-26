//! Bytecode definition and VM for bytecode execution.

mod builder;
pub mod bytecode;
pub mod op;

#[cfg(test)]
mod tests;

use std::rc::Rc;

pub use self::builder::Builder;

use data;
use data::Address;
use data::Literal;
use env::EnvStack;
use errors::*;
use syscall;
use vm::bytecode::Bytecode;
use vm::op::Op;

#[derive(Clone, Debug)]
pub enum VMState {
    Done(Literal),
    Stopped,
    Running,
    RunningUntil(usize),
}

impl VMState {
    /// Can we permit execution while in this vm state?
    fn can_run(&self) -> bool {
        match self {
            VMState::Running => true,
            VMState::RunningUntil(i) if *i > 0 => true,
            _ => false,
        }
    }

    fn check_run(&mut self) -> Result<()> {
        if let VMState::RunningUntil(0) = self {
            *self = VMState::Stopped;
        }

        if self.can_run() {
            Ok(())
        } else {
            Err(format_err!("Cannot execute VM while in state {:?}", self))
        }
    }

    fn get_ret(&self) -> Option<Literal> {
        if let VMState::Done(ref l) = self {
            Some(l.clone())
        } else {
            None
        }
    }

    fn cost(&mut self, cost: usize) {
        if let VMState::RunningUntil(ref mut c) = self {
            *c = c.saturating_sub(cost);
        }

        if let VMState::RunningUntil(0) = self {
            *self = VMState::Stopped;
        }
    }
}

#[derive(Debug)]
pub struct VMConfig {
    reset_on_error: bool,
    print_trace: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        VMConfig {
            reset_on_error: true,
            print_trace: false,
        }
    }
}

/// A non-reusable bytecode VM.
///
/// Keeps track of data stack, frame stack, environment stack, and the code.
#[derive(Debug)]
pub struct VM {
    /// The live code repo.
    pub code: Bytecode,
    pub frames: Vec<data::Address>,
    /// The data stack
    pub stack: Vec<data::Literal>,
    sys: syscall::SyscallRegistry,
    /// The current local environment bindings.
    pub environment: EnvStack,
    pub state: VMState,
    conf: VMConfig,
}

impl VM {
    /// Create a VM loaded with the provided code. Program counter is initially `(0, 0)`.
    pub fn new(code: Bytecode) -> VM {
        let mut b = Builder::new();

        b.code(code).default_libs();

        b.build()
    }

    fn pcounter(&mut self) -> Result<Address> {
        let pc = self
            .frames
            .last_mut()
            .ok_or_else(|| err_msg("Stack empty, no counter"))?;
        let a = *pc;

        data::address_inc(pc);

        Ok(a)
    }

    /// Step until a "top-level" return, which is when the frame stack is empty.
    /// At this point, the stack is popped and returned. A failure to pop a value
    /// is treated as an error state. Propagates errors from [`VM::single_step()`]. If
    /// `print` is `true`, print the VM state on every state.
    pub fn step_until_value(&mut self) -> Result<data::Literal> {
        if self.state.can_run() {
            return Err(err_msg("Already running"));
        }

        self.state = VMState::Running;

        if let Err(e) = self.state_step() {
            if self.conf.reset_on_error {
                self.reset_exec();
            }
            return Err(e.context("While stepping until return").into());
        }

        self.state
            .get_ret()
            .ok_or_else(|| err_msg("No return value"))
    }

    /// Step until a resource is consumed. Each operation executed decrements a counter
    /// initially set to `max`. As with [`VM::step_until_value()`], the lack of a return value
    /// is treated as an error.
    ///
    /// Returns `Err` if an error is encountered
    ///
    /// `Ok(None)` if the resource pool was exhausted
    ///
    /// `Ok(Some(_))` if there was a top level return.
    pub fn step_until_cost(&mut self, max: usize) -> Result<Option<data::Literal>> {
        if self.state.can_run() {
            return Err(err_msg("Already running"));
        }

        self.state = VMState::RunningUntil(max);

        self.state_step().context("While stepping until cost")?;

        Ok(self.state.get_ret())
    }

    /// Step until the VM can no longer run.
    ///
    /// See [`step_until_cost()`] and [`step_until_value()`] for methods
    /// that ensure the VM can be set to a running state, and then set it,
    /// because `state_step` doesn't do that.
    pub fn state_step(&mut self) -> Result<()> {
        while self.state.can_run() {
            self.single_step().context("Stepping in state_step")?;

            if self.frames.is_empty() {
                let res = self
                    .stack
                    .pop()
                    .ok_or_else(|| err_msg("Frames empty, but no value to return"))?;

                self.state = VMState::Done(res);
            }
        }

        Ok(())
    }

    /// Manually jump the VM to an address. This returns an `Err` if the frame
    /// stack is empty.
    pub fn jump(&mut self, addr: data::Address) -> Result<()> {
        let pc: &mut data::Address = self
            .frames
            .last_mut()
            .ok_or_else(|| err_msg("Frames empty, no way to jump"))?;

        *pc = addr;
        Ok(())
    }

    /// Loads new code into the VM, and resets the data and frame stack.
    pub fn reset(&mut self, code: Bytecode) {
        self.code = code;
        self.stack = vec![];
        self.frames = vec![(0, 0)];
        self.state = VMState::Stopped;
    }

    /// Reset the execution state, throwing the existing data stack, frame stack, and state.
    pub fn reset_exec(&mut self) {
        self.stack = vec![];
        self.frames = vec![];
        self.state = VMState::Stopped;
    }

    /// Imports new code into the VM's [`Bytecode`] repo, jumps to the main
    /// function of the new code, and returns that address.
    ///
    /// This clears the frame stack, and shouldn't be used mid-execution.
    pub fn import_jump(&mut self, code: &Bytecode) -> Address {
        let a = self.code.import(code);
        self.frames.clear();
        self.frames.push(a);
        a
    }

    fn invoke_syscall(stack: &mut Vec<Literal>, syscall: &syscall::Syscall) -> Result<()> {
        use syscall::Syscall;
        match syscall {
            Syscall::Stack(ref f) => f(stack),
            Syscall::A1(ref f) => {
                let a = stack
                    .pop()
                    .ok_or_else(|| err_msg("Error popping stack for 1-arity syscall"))?;
                let v = f(a).context("While executing 1-arity syscall")?;
                stack.push(v);
                Ok(())
            }
            Syscall::A2(ref f) => {
                let a = stack.pop().ok_or_else(|| {
                    err_msg("Error popping stack for first arg of 2-arity syscall")
                })?;
                let b = stack.pop().ok_or_else(|| {
                    err_msg("Error popping stack for second arg of 2-arity syscall")
                })?;
                let v = f(a, b).context("While executing 2-arity syscall")?;
                stack.push(v);
                Ok(())
            }
        }
    }

    /// Execute a single operation. Returns an `Err` if an error was encountered,
    /// or `Ok(())` if it was successful. No particular attempt has been made to make
    /// `Err`s survivable, but no particular attempt has been made to prevent further
    /// execution. No attempt has been made to attempt to maintain operation arity in
    /// error states. See `fn op_*` for raw implementations, and see  [ `Op` ]
    /// for high level descriptions of the operations.
    pub fn single_step(&mut self) -> Result<()> {
        self.state.check_run().context("While single stepping")?;

        let pc = self.pcounter()?;
        // TODO: maybe don't look up program chunk first?
        let op = match self.code.addr(pc) {
            Ok(x) => x,
            Err(e) => {
                // TODO: This should only happen when chunk lookup fails
                // Fix this when real error states are implemented.
                if let Some(ref f) = self.sys.lookup(pc) {
                    VM::invoke_syscall(&mut self.stack, f).context(format!(
                        "Invoking syscall {:?}, with stack {:?}",
                        pc, self.frames
                    ))?;

                    self.state.cost(self.sys.cost(pc));

                    self.frames
                        .pop()
                        .ok_or_else(|| err_msg("Error popping stack after syscall"))?;
                    return Ok(());
                }
                // This is required because we can't return a context directly
                Err(e).context("builtin lookup failed")?;
                return Ok(()); // this never exeuctes
            }
        };

        self.state.cost(op.cost());

        if self.conf.print_trace {
            println!("Trace: {:?}", self);
        }

        self.exec_op(op)
            .context(format_err!("While executing at {:?}", pc))?;

        Ok(())
    }

    // Below here, we don't care about the state, vis a vie whether we execute
    // or not, or whether we incur costs. We only care about it to adjust execution
    // flow as a user would, to await data or halt execution.

    /// Execute a single operation, ignoring any already loaded code and ignoring the
    /// program counter. See [`VM::single_step()`] for more details.
    pub fn exec_op(&mut self, op: Op) -> Result<()> {
        // https://users.rust-lang.org/t/announcing-failure/13895/18
        match op {
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
            Op::MakeClosure => self
                .op_make_closure()
                .context("Executing operation make-closujre")?,
            Op::CallArity(a) => self
                .op_call_arity(a)
                .context("Executing operation call-arity")?,
        }
        Ok(())
    }

    fn op_lit(&mut self, l: data::Literal) -> Result<()> {
        self.stack.push(l);
        Ok(())
    }

    fn op_return(&mut self) -> Result<()> {
        self.frames
            .pop()
            .ok_or_else(|| err_msg("Attempted to return on empty stack"))?;
        Ok(())
    }

    fn op_call(&mut self) -> Result<()> {
        let a = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop data stack for jump"))?;

        let addr = match a {
            Literal::Address(addr) => addr,
            Literal::Closure(_, addr) => addr,
            _ => return Err(err_msg("attempted to jump to non-address")),
        };

        self.frames.push(addr);
        Ok(())
    }

    fn op_jump(&mut self) -> Result<()> {
        let address = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for address"))?
            .ensure_address()?;

        self.jump(address)
    }

    // Currently, this doesn't always consume 3 stack items.
    // This may need to change.
    fn op_jumpcond(&mut self) -> Result<()> {
        let cond = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for conditional for if zero"))?;

        let then = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for address for if true"))?
            .ensure_address()?;

        let els = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for address for if false"))?
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
            .ok_or_else(|| err_msg("Attempted to pop stack for keyword for load"))?
            .ensure_keyword()?;

        let mut val = self.environment.get(&keyword)?;

        // TODO:
        //let val = Rc::make_mut(&mut val);

        self.stack.push(val.clone());
        Ok(())
    }

    fn op_store(&mut self) -> Result<()> {
        let keyword = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for keyword for store"))?
            .ensure_keyword()?;
        let value = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for value for store"))?;

        self.environment.insert(keyword, value)?;

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
            .ok_or_else(|| err_msg("Attmempted to dup empty stack"))?
            .clone();
        self.stack.push(v);
        Ok(())
    }

    fn op_pop(&mut self) -> Result<()> {
        self.stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop empty stack"))?;
        Ok(())
    }

    fn op_make_closure(&mut self) -> Result<()> {
        let arity = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop empty stack"))?
            .ensure_number()?;
        let address = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop empty stack"))?
            .ensure_address()?;
        self.stack.push(Literal::Closure(arity as usize, address));

        Ok(())
    }

    fn op_call_arity(&mut self, a: usize) -> Result<()> {
        let c = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop data stack for jump"))?;

        let addr = match c {
            Literal::Address(addr) => addr,
            Literal::Closure(_, addr) => addr,
            _ => return Err(err_msg("attempted to jump to non-address")),
        };

        if let Literal::Closure(arity, _) = c {
            if arity != a {
                return Err(format_err!(
                    "Attempted to call closure with arity {:} with argument arity {:}",
                    arity,
                    a
                ));
            }
        }

        self.frames.push(addr);

        Ok(())
    }
}
