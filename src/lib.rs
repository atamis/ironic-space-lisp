
pub mod vm {
    use std::fmt;

    pub mod data {
        pub type Literal = u32;
    }

    pub enum Op {
        Lit(data::Literal),
        PlusOp,
        ApplyFunction(Box<Function>),
    }

    impl fmt::Debug for Op {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            match self {
                Op::ApplyFunction(_) => write!(f, "ApplyFunction"),
                Op::PlusOp => write!(f, "PlusOp"),
                Op::Lit(i) => write!(f, "Lit({})", i),
            }
        }
    }

    pub trait Function {
        fn get_arity(&self) -> usize;
        fn invoke(&self, stack: &mut Vec<data::Literal>);
    }

    #[derive(Debug)]
    pub struct AdditionFunction;

    impl Function for AdditionFunction {
        fn get_arity(&self) -> usize {
            2
        }

        fn invoke(&self, stack: &mut Vec<data::Literal>) {
            let x = stack.pop().unwrap();
            let y = stack.pop().unwrap();
            let s = x + y;
            stack.push(s);
        }
    }

    #[derive(Debug)]
    pub struct VM {
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
                frames: vec![frame]
            }
        }

        pub fn single_step(&mut self) {
            let frame = self.frames.last_mut().expect("Looks like we're done");
            let op = &frame.instructions[frame.idx];
            frame.idx += 1;

            match op {
                Op::Lit(l) => frame.stack.push(l.clone()),
                Op::PlusOp => {
                    let x = frame.stack.pop().unwrap();
                    let y = frame.stack.pop().unwrap();
                    let s = x + y;
                    frame.stack.push(s);
                }
                Op::ApplyFunction(f) => f.invoke(&frame.stack),
            }
        }
    }
}
