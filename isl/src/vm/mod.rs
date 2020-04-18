//! Bytecode definition and VM for bytecode execution.

mod builder;
pub mod bytecode;
pub mod op;

#[cfg(test)]
mod tests;

pub use self::builder::Builder;

use crate::data;
use crate::data::Address;
use crate::data::Literal;
use crate::env::EnvStack;
use crate::errors::*;
use crate::exec;
use crate::exec::ExecHandle;
use crate::syscall;
use crate::vm::bytecode::Bytecode;
use crate::vm::op::Op;

/// Enum representing the different states a [`VM`] can be in.
///
/// Methods on the enum represent some internal state transitions
/// useful to the running [`VM`]. In particular, the [`VM`] depends
/// on some of these methods to control its execution flow.
#[derive(Clone, Debug, PartialEq)]
pub enum VMState {
    /// The [`VM`] is done, and has a return value.
    Done(Literal),
    /// The [`VM`] is paused, ready for more execution at a later time.
    Stopped,
    /// The [`VM`] is actively running. It is usually more efficient to utilize
    /// [`RunningUntil`](VMState::RunningUntil) to control [`VM`] execution.
    /// This can be used to start the [`VM`] for an indefinite run, although
    /// [`step_until_value`](VM::step_until_value) does a lot of the housework
    /// for you.
    Running,
    /// The [`VM`] is actively running, but depleting a resource with every cycle
    /// and syscall. The remaining resource reserve is indicated with the tuple field.
    /// [`step_until_cost`](VM::step_until_cost) does a lot of the housework for you.
    RunningUntil(usize),
    /// The [`VM`] is waiting for a message. This means the [`VM`] will refuse to execute
    /// until it leaves this state. To supply a message and leave this state, see
    /// [`answer_waiting`](VM::answer_waiting)
    Waiting,
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

    /// Returns whether we can run or not as a result, `Ok(())` for yes, `Err(_)` for no.
    ///
    /// Additionally, if the `VMState` is [`RunningUntil`](VMState::RunningUntil), it
    /// will reset to [`Stopped`](VMState::Stopped) if the remaining cost is 0.
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

    /// If there is a return value available, clone and return it. Otherwise return `None`.
    pub fn get_ret(&self) -> Option<Literal> {
        if let VMState::Done(ref l) = self {
            Some(l.clone())
        } else {
            None
        }
    }

    /// Incur a cost on the cost reserve when in [`VMState::RunningUntil`].
    ///
    /// If the remaining cost reserve is 0, reset the state to [`Stopped`](VMState::Stopped).
    fn cost(&mut self, cost: usize) {
        if let VMState::RunningUntil(ref mut c) = self {
            *c = c.saturating_sub(cost);
        }

        if let VMState::RunningUntil(0) = self {
            *self = VMState::Stopped;
        }
    }
}

/// Configuration options for VMs.
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// When using [`step_until_cost`](VM::step_until_cost), should the VM reset if it encounters an error?
    ///
    /// Default: `true`
    pub reset_on_error: bool,
    /// Should the VM print the VM state after every cycle?
    ///
    /// Default: `false`
    pub print_trace: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        VMConfig {
            reset_on_error: true,
            print_trace: false,
        }
    }
}

/// Stack frame used by the VM.
///
/// Consists of an address and a vector of local variables.
#[derive(Debug, Clone)]
pub struct Frame {
    addr: data::Address,
    locals: Vec<Literal>,
}

impl Frame {
    /// Create a new frame with an address and an empty set of locals.
    pub fn new(addr: data::Address) -> Frame {
        Frame {
            addr,
            locals: vec![],
        }
    }
}

/// A non-reusable bytecode VM.
///
/// Keeps track of data stack, frame stack, environment stack, and the code.
#[derive(Debug, Clone)]
pub struct VM {
    /// The live code repo.
    pub code: Bytecode,
    /// The call stack of the VM. The VM treats the top of the stack as the program pointer.
    /// Using the [`Call`](op::Op::Call) operation pushes the address to the top of the frame stack.
    pub frames: Vec<Frame>,
    /// The data stack
    pub stack: Vec<data::Literal>,
    sys: syscall::SyscallRegistry,
    /// The current local environment bindings.
    pub environment: EnvStack,
    /// The current state of the VM. See [`VMState`] for more information.
    pub state: VMState,
    conf: VMConfig,
    /// This fields contains an optional [`ExecHandle`](exec::ExecHandle) the VM uses to interface with the execution environment.
    pub proc: Option<Box<exec::RouterHandle>>,
}

impl VM {
    /// Create a VM loaded with the provided code. Program counter is initially `(0, 0)`.
    pub fn new(code: Bytecode) -> VM {
        let mut b = Builder::new();

        b.code(code).default_libs();

        b.build()
    }

    fn pcounter(&mut self) -> Result<Address> {
        let pc = &mut self
            .frames
            .last_mut()
            .ok_or_else(|| err_msg("Stack empty, no counter"))?
            .addr;
        let a = *pc;

        data::address_inc(pc);

        Ok(a)
    }

    /// Step until a "top-level" return, which is when the frame stack is empty.
    /// At this point, the stack is popped and returned. A failure to pop a value
    /// is treated as an error state. Propagates errors from [`VM::single_step()`].
    ///
    /// Warning: this doesn't handle waiting properly.
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
    /// See [`VM::step_until_cost`] and [`VM::step_until_value`] for methods
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
        let pc: &mut data::Address = &mut self
            .frames
            .last_mut()
            .ok_or_else(|| err_msg("Frames empty, no way to jump"))?
            .addr;

        *pc = addr;
        Ok(())
    }

    /// Loads new code into the VM, and resets the data and frame stack.
    pub fn reset(&mut self, code: Bytecode) {
        self.code = code;
        self.stack = vec![];
        self.frames = vec![Frame::new((0, 0))];
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
        self.frames.push(Frame::new(a));
        a
    }

    fn invoke_syscall(stack: &mut Vec<Literal>, syscall: &syscall::Syscall) -> Result<()> {
        use crate::syscall::Syscall;
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
            Syscall::A3(ref f) => {
                let arg1 = stack.pop().ok_or_else(|| {
                    err_msg("Error popping stack for first arg of 2-arity syscall")
                })?;
                let arg2 = stack.pop().ok_or_else(|| {
                    err_msg("Error popping stack for second arg of 2-arity syscall")
                })?;
                let arg3 = stack.pop().ok_or_else(|| {
                    err_msg("Error popping stack for third arg of 2-arity syscall")
                })?;
                let v = f(arg1, arg2, arg3).context("While executing 3-arity syscall")?;
                stack.push(v);
                Ok(())
            }
        }
    }

    /// Answer a waiting VM with a message.
    ///
    /// When in [`VMState::Waiting`], the VM is waiting for a message in the form
    /// of a [`Literal`]. If the VM is not in the waiting state, this returns an
    /// `Err`.
    pub fn answer_waiting(&mut self, a: Literal) -> Result<()> {
        if self.state != VMState::Waiting {
            return Err(format_err!(
                "Can't answer waiting when in state {:?}",
                self.state
            ));
        }

        self.stack.push(a);
        self.state = VMState::Stopped;

        Ok(())
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
                .context("Executing operation make-closure")?,
            Op::CallArity(a) => self
                .op_call_arity(a)
                .context("Executing operation call-arity")?,
            Op::Wait => self.op_wait().context("Executing operation wait")?,
            Op::Send => self.op_send().context("Executing operation send")?,
            Op::Fork => self.op_fork().context("Executing operation fork")?,
            Op::Pid => self.op_pid().context("Executing operation pid")?,
            Op::Watch => self.op_watch().context("Executing operation watch")?,
            Op::LoadLocal(i) => self
                .op_load_local(i)
                .context("Executing operation load local")?,
            Op::StoreLocal(i) => self
                .op_store_local(i)
                .context("Executing operation store local")?,
            Op::LoadPool(i) => self
                .op_load_pool(i)
                .context("Executing operation load pool")?,
            Op::Terminate => self
                .op_terminate()
                .context("Executing operation terminate")?,
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
            _ => return Err(err_msg(format!("attempted to jump to non-address {:?}", a))),
        };

        self.frames.push(Frame::new(addr));
        Ok(())
    }

    fn op_jump(&mut self) -> Result<()> {
        let address = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for address"))?
            .ensure_address_flexible()?;

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
        let symbol = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for Symbol for load"))?
            .ensure_symbol()?;

        let val = self.environment.get(&symbol)?;

        self.stack.push(val.clone());
        Ok(())
    }

    fn op_store(&mut self) -> Result<()> {
        let symbol = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for Symbol for store"))?
            .ensure_symbol()?;
        let value = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for value for store"))?;

        self.environment.insert(symbol, value)?;

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
            _ => return Err(err_msg(format!("attempted to jump to non-address {:?}", c))),
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

        self.frames.push(Frame::new(addr));

        Ok(())
    }

    fn op_wait(&mut self) -> Result<()> {
        self.state = VMState::Waiting;
        Ok(())
    }

    fn op_send(&mut self) -> Result<()> {
        let pid = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for send destination"))?
            .ensure_pid()?;
        let msg = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for message to send"))?;

        let proc = self
            .proc
            .as_mut()
            .ok_or_else(|| err_msg("Sending without procinfo"))?;

        proc.send(pid, msg)?;

        self.stack.push(pid.into());

        Ok(())
    }

    fn op_pid(&mut self) -> Result<()> {
        if let Some(ref mut proc) = self.proc {
            self.stack.push(proc.get_pid().into());
            Ok(())
        } else {
            self.stack.push(false.into());
            Ok(())
        }
    }

    fn op_fork(&mut self) -> Result<()> {
        let mut new_vm = self.clone();
        new_vm.stack.push(true.into());
        self.stack.push(false.into());
        self.proc
            .as_mut()
            .expect("Forking requires registered ExecHandler")
            .spawn(new_vm)?;

        Ok(())
    }

    fn op_watch(&mut self) -> Result<()> {
        let watched = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for watch target"))?
            .ensure_pid()?;

        self.proc
            .as_mut()
            .expect("Watching requires registered ExecHandler")
            .watch(watched)?;

        self.stack.push(watched.into());

        Ok(())
    }

    fn local_cap_ref(&mut self, index: usize) -> Result<&mut Literal> {
        {
            let locals = &mut self
                .frames
                .last_mut()
                .ok_or_else(|| err_msg("Stack empty, no locals"))?
                .locals;

            while locals.len() <= index {
                locals.push(false.into());
            }
        }

        Ok(&mut self.frames.last_mut().unwrap().locals[index])
    }

    fn op_load_local(&mut self, index: usize) -> Result<()> {
        let val = self.local_cap_ref(index)?.clone();

        self.stack.push(val);

        Ok(())
    }

    fn op_store_local(&mut self, index: usize) -> Result<()> {
        let msg = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for value to store locally"))?;

        let local_ref = self.local_cap_ref(index)?;

        *local_ref = msg;

        Ok(())
    }

    fn op_load_pool(&mut self, index: usize) -> Result<()> {
        println!("{:?}", self.code.pool);

        self.stack.push(
            self.code
                .pool
                .get(index)
                .ok_or_else(|| err_msg(format!("Loading from pool index {:}", index)))?
                .clone(),
        );

        Ok(())
    }

    fn op_terminate(&mut self) -> Result<()> {
        let ret = self
            .stack
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop stack for terminate value"))?;

        self.frames.clear();
        self.stack.clear();
        self.stack.push(ret);
        Ok(())
    }
}
