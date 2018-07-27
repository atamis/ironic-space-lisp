
pub mod vm {
    use std::fmt;
    use std::mem;

    pub mod data {
        use std::rc::Rc;
        #[derive(Debug, Clone)]
        pub enum Literal {
            Number(u32),
            Builtin(Rc<super::BuiltinFunction>),
            Lambda(Rc<super::LambdaFunction>)
        }

        impl Literal {
            pub fn expect_number(&self) -> u32 {
                if let Literal::Number(n) = self {
                    return *n
                } else {
                    panic!("Expected number, got {:?}", self)
                }
            }
        }

    }

    #[derive(Debug)]
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

    impl VM {
        pub fn new(instructions: Vec<Op>) -> VM {
            let frame = StackFrame {instructions, idx: 0, stack: Vec::new()};
            VM {
                return_value: None,
                frames: vec![frame],
            }
        }

        pub fn step_until_value(&mut self, print: bool) -> &data::Literal {
            loop {
                if let Some(ref r) = self.return_value {
                    return &r
                }

                if print {
                    println!("{:?}", self);
                }

                self.single_step();
            }
        }

        pub fn single_step(&mut self) {

            let mut is_return = false;
            let mut new_frame: Option<StackFrame> = None;

            {
                let frame = self.frames.last_mut().expect("Looks like we're done");
                let instructions = &frame.instructions;
                let op = &instructions[frame.idx];
                frame.idx += 1;

                match op {
                    Op::Lit(l) => frame.stack.push(( *l ).clone()),
                    Op::PlusOp => {
                        let x = frame.stack.pop().unwrap().expect_number();
                        let y = frame.stack.pop().unwrap().expect_number();
                        let s = x + y;
                        frame.stack.push(data::Literal::Number(s));
                    }
                    Op::ApplyFunction => {
                        let function = frame.stack.pop().unwrap();

                        match function {
                            data::Literal::Builtin(f) => {
                                f.invoke(&mut frame.stack);
                            },
                            data::Literal::Lambda(f) => {
                                let mut new_stack: Vec<data::Literal> = Vec::new();
                                for _ in 0..f.get_arity() {
                                    new_stack.push(frame.stack.pop().unwrap());
                                }
                                new_frame = Some(StackFrame {
                                    instructions: f.get_instructions(),
                                    idx: 0,
                                    stack: new_stack
                                });
                            },
                            _ => panic!("Attempted to apply non-function"),
                        }

                    },
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
                let return_val = mem::replace(&mut last_frame.stack.last().unwrap(), &data::Literal::Number(0));

                match self.frames.last_mut() {
                    Some(ref mut next_frame) => {
                        next_frame.stack.push(( *return_val ).clone());
                    }
                    None => {
                        self.return_value = Some(( *return_val ).clone());
                    }
                }

            }

        }
    }
}
