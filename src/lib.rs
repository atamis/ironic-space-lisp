pub mod data;
pub mod error;

pub mod vm {
    use std::fmt;
    use std::mem;
    use std::rc::Rc;

    use error::VmGeneralError;
    use data::Lisp;
    use data::Op;

    #[deprecated]
    pub fn normal_eval<'a>(l: &Lisp) -> Lisp {
        match l {
            Lisp::List(rc) => {
                let l: Vec<Lisp> = rc.iter().map(normal_eval).collect();
                let op = &l[0];
                let args = &l[1..];

                match op {
                    Lisp::Op(Op::Add) => {
                        let sum = args.iter().fold(0, |sum, i| match i {
                            Lisp::Num(i) => sum + i,
                            _ => panic!("Can't add non-numbers"),
                        });
                        Lisp::Num(sum)
                    }
                    _ => panic!("Not operation, or operation not implemented"),
                }
            }
            x => (*x).clone(),
        }
    }

    #[derive(Debug)]
    enum FrameStepResult {
        Continue,
        Return(Lisp),
        Recur(Lisp),
    }

    trait Frame: fmt::Debug {
        fn single_step(
            &mut self,
            return_val: &mut Option<Lisp>,
        ) -> Result<FrameStepResult, VmGeneralError>;
    }

    #[derive(Debug)]
    pub struct ValueFrame {
        lisp: Lisp,
    }

    impl Frame for ValueFrame {
        fn single_step(
            &mut self,
            _return_val: &mut Option<Lisp>,
        ) -> Result<FrameStepResult, VmGeneralError> {
            match &self.lisp {
                Lisp::List(_) => Err(VmGeneralError),
                x => Ok(FrameStepResult::Return(x.clone())),
            }
        }
    }

    #[derive(Debug)]
    pub struct ApplicationFrame {
        list: Vec<Lisp>,
        vals: Vec<Lisp>,
    }

    impl ApplicationFrame {
        pub fn new(lisp: Lisp) -> ApplicationFrame {
            match lisp {
                Lisp::List(l) => {
                    let list = Rc::try_unwrap(l).unwrap();
                    ApplicationFrame {
                        list: list,
                        vals: Vec::new(),
                    }
                }
                _ => panic!(
                    "Attempted to make ApplicationFrame with lisp that wasn't an application"
                ),
            }
        }
    }

    impl Frame for ApplicationFrame {
        fn single_step(
            &mut self,
            return_val: &mut Option<Lisp>,
        ) -> Result<FrameStepResult, VmGeneralError> {
            if let Some(_) = return_val {
                if let Some(myr) = mem::replace(return_val, None) {
                    self.vals.push(myr);
                }
            }

            if self.list.len() == 0 {
                let op = &self.vals[0];
                let args = &self.vals[1..];

                match op {
                    Lisp::Op(Op::Add) => {
                        let sum = args.iter().fold(0, |sum, i| match i {
                            Lisp::Num(i) => sum + i,
                            _ => panic!("Can't add non-numbers"),
                        });
                        return Ok(FrameStepResult::Return(Lisp::Num(sum)));
                    }
                    _ => panic!("Not operation, or operation not implemented"),
                }
            } else {
                let l = self.list.remove(0); // TODO: use pop and reverse arg list

                return Ok(FrameStepResult::Recur(l));
            }
        }
    }

    fn match_frame(lisp: Lisp) -> Box<Frame> {
        match lisp {
            Lisp::List(_) => Box::new(ApplicationFrame::new(lisp)),
            x => Box::new(ValueFrame { lisp: x }),
        }
    }

    #[derive(Debug)]
    pub struct Evaler {
        return_value: Option<Lisp>,
        frames: Vec<Box<Frame>>,
    }

    impl Evaler {
        pub fn new(lisp: Lisp) -> Evaler {
            Evaler {
                frames: vec![match_frame(lisp)],
                return_value: None,
            }
        }

        pub fn is_done(&self) -> bool {
            self.frames.len() == 0
        }

        pub fn step_until_return(&mut self) -> Result<Option<Lisp>, VmGeneralError> {
            while !self.is_done() {
                println!("{:?}", self);

                self.single_step()?;
            }

            return Ok(self.return_value.clone());
        }

        pub fn single_step(&mut self) -> Result<(), VmGeneralError> {
            let mut pop_frame = false;
            let mut new_frame = None;

            {
                let frame = self.frames.last_mut().unwrap();

                let fsr = frame.single_step(&mut self.return_value)?;

                match fsr {
                    FrameStepResult::Continue => (),
                    FrameStepResult::Return(l) => {
                        self.return_value = Some(l);
                        pop_frame = true;
                    }
                    FrameStepResult::Recur(l) => {
                        self.return_value = None;
                        new_frame = Some(match_frame(l))
                    }
                }
            }

            if pop_frame {
                self.frames.pop();
            }

            if let Some(f) = new_frame {
                self.frames.push(f);
            }
            Ok(())
        }
    }
}
