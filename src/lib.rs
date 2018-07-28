pub mod data;
pub mod error;
pub mod lisp;

pub mod vm {
    use std::fmt;
    use std::mem;

    use data;
    use error;

    #[derive(Debug, Clone)]
    pub enum Op {
        Lit(data::Literal),
        ReturnOp,
        PlusOp,
        ApplyFunction,
    }

    // Bad hack here
    pub trait BuiltinFunction: fmt::Debug {
        fn get_arity(&self) -> usize;
        fn invoke(&self, stack: &mut Vec<data::Literal>);
    }

    // Same bad hack
    pub trait LambdaFunction: fmt::Debug {
        fn get_arity(&self) -> usize;
        fn get_instructions(&self) -> Vec<Op>;
    }

    #[derive(Debug)]
    pub struct AdditionFunction;

    impl BuiltinFunction for AdditionFunction {
        fn get_arity(&self) -> usize {
            2
        }

        fn invoke(&self, stack: &mut Vec<data::Literal>) {
            // TODO: maybe make this return result?
            let x = stack.pop().unwrap().expect_number();
            let y = stack.pop().unwrap().expect_number();
            let s = x + y;
            stack.push(data::Literal::Number(s));
        }
    }

    #[derive(Debug)]
    pub struct AddOneFunction;

    impl LambdaFunction for AddOneFunction {
        fn get_arity(&self) -> usize {
            1
        }

        fn get_instructions(&self) -> Vec<Op> {
            vec![Op::Lit(data::Literal::Number(1)), Op::PlusOp, Op::ReturnOp]
        }
    }

    #[derive(Debug)]
    pub struct VM {
        return_value: Option<data::Literal>,
        frames: Vec<StackFrame>,
    }

    #[derive(Debug)]
    struct StackFrame {
        instructions: Vec<Op>,
        idx: usize,
        stack: Vec<data::Literal>,
    }

    impl StackFrame {
        pub fn new(instructions: Vec<Op>, initial_stack: Vec<data::Literal>) -> StackFrame {
            StackFrame {
                instructions,
                idx: 0,
                stack: initial_stack,
            }
        }

        pub fn next_instruction(&mut self) -> Op {
            let op = &self.instructions[self.idx];
            self.idx += 1;
            op.clone()
        }

        pub fn stack_pop(&mut self) -> Result<data::Literal, error::VmPopError> {
            match self.stack.pop() {
                Some(x) => Ok(x),
                None => Err(error::VmPopError),
            }
        }
    }

    impl VM {
        pub fn new(instructions: Vec<Op>) -> VM {
            let frame = StackFrame::new(instructions, Vec::new());
            VM {
                return_value: None,
                frames: vec![frame],
            }
        }

        pub fn step_until_value(&mut self, print: bool) -> Result<&data::Literal, error::VmError> {
            loop {
                if let Some(ref r) = self.return_value {
                    return Ok(&r);
                }

                if print {
                    println!("{:?}", self);
                }

                self.single_step()?;
            }
        }

        pub fn single_step(&mut self) -> Result<(), error::VmError> {
            let mut is_return = false;
            let mut new_frame: Option<StackFrame> = None;

            {
                let frame = self.frames.last_mut().expect("Looks like we're done");
                let op = frame.next_instruction();

                match op {
                    Op::Lit(l) => frame.stack.push((l).clone()),
                    Op::PlusOp => {
                        let x = frame.stack_pop()?.ensure_number()?;
                        let y = frame.stack_pop()?.ensure_number()?;
                        let s = x + y;
                        frame.stack.push(data::Literal::Number(s));
                    }
                    Op::ApplyFunction => {
                        let function = frame.stack_pop()?;

                        match function {
                            data::Literal::Builtin(f) => {
                                f.invoke(&mut frame.stack);
                            }
                            data::Literal::Lambda(f) => {
                                let mut new_stack: Vec<
                                    data::Literal,
                                > = Vec::new();
                                for _ in 0..f.get_arity() {
                                    new_stack.push(frame.stack_pop()?);
                                }
                                new_frame = Some(StackFrame::new(f.get_instructions(), new_stack))
                            }
                            _ => panic!("Attempted to apply non-function"),
                        }
                    }
                    Op::ReturnOp => {
                        is_return = true;
                    }
                }
            }

            if let Some(f) = new_frame {
                self.frames.push(f);
            }

            if is_return {
                let last_frame = self.frames.pop().unwrap();
                let return_val = mem::replace(
                    &mut last_frame.stack.last().unwrap(),
                    &data::Literal::Number(0),
                );

                match self.frames.last_mut() {
                    Some(ref mut next_frame) => {
                        next_frame.stack.push((*return_val).clone());
                    }
                    None => {
                        self.return_value = Some((*return_val).clone());
                    }
                }
            }

            return Ok(());
        }
    }
}
