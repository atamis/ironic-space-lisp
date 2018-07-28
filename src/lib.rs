/// Contains global data definitions for Ironic Space Lisp.
///
/// These data structures are used to represent both code/AST and live data.
/// See `::vm` for evaluators using these structures.
pub mod data;

/// Contains out of date error types and a lot of boilerplate.
pub mod error;

/// Ironic Space Lisp VM.
///
/// Contains evaluator for Ironic Space Lisp. Utilizes data structures from
/// `::data` and errors from `::error`.
pub mod vm {
    use std::fmt;
    use std::mem;
    use std::rc::Rc;

    use error::VmGeneralError;
    use data::Lisp;
    use data::Op;

    /// Normal recursive eval for lisp.
    ///
    /// This is more of an example of just how easy this stuff is to write in a
    /// normal recursive mode. Panics aggressively.
    #[deprecated]
    pub fn normal_eval<'a>(l: &Lisp) -> Lisp {
        match l {
            Lisp::List(rc) => {
                // Main recursion here:
                let l: Vec<Lisp> = rc.iter().map(normal_eval).collect();
                let op = &l[0];
                let args = &l[1..];

                match op {
                    Lisp::Op(Op::Add) => {
                        // Sum up the args and aggressively panic.
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

    // Controls flow in evaluation.
    #[derive(Debug)]
    enum FrameStepResult {
        // Don't do anything to the control flow.
        Continue,
        // Indicates that the fragment is done and wants to return a value. Set
        // the return value, and pop the frame stack.
        Return(Lisp),
        // Start to recur on a another piece of lisp code.
        Recur(Lisp),
    }

    // Requires debug so we can print trait objects.
    trait Frame: fmt::Debug {
        // Evaluate a single step on the current fragment. Takes a mutable
        // reference to the return val so the fragment can claim the relevant
        // return values from recurring. You can't set the normal return value,
        // you have to return via FrameStepResult.
        fn single_step(
            &mut self,
            return_val: &mut Option<Lisp>,
        ) -> Result<FrameStepResult, VmGeneralError>;
    }

    // Represents a single value fragment, usually a data literal.
    #[derive(Debug)]
    struct ValueFrame {
        lisp: Lisp,
    }

    impl Frame for ValueFrame {
        // Currently handles numbers and ops.
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


    // Will represent function application, currently handles operations.
    #[derive(Debug)]
    struct ApplicationFrame {
        // Lisp terms to eval
        list: Vec<Lisp>,
        // already evaled args.
        vals: Vec<Lisp>,
    }

    impl ApplicationFrame {
        pub fn new(lisp: Lisp) -> ApplicationFrame {
            match lisp {
                Lisp::List(l) => {
                    // TODO: maybe don't do this.
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
        // This function is basically backwards.
        fn single_step(
            &mut self,
            return_val: &mut Option<Lisp>,
        ) -> Result<FrameStepResult, VmGeneralError> {
            // Extract the result of the last fragment we recurred on.
            if let Some(_) = return_val {
                if let Some(myr) = mem::replace(return_val, None) {
                    self.vals.push(myr);
                }
            }

            // We've evaled all the arg fragments, so it's time to actually
            // apply the args to the operation.
            if self.list.len() == 0 {
                let op = &self.vals[0];
                let args = &self.vals[1..];

                match op {
                    Lisp::Op(Op::Add) => {
                        // Sum everything up, and aggressively panic if we can't
                        // add correctly.
                        let sum = args.iter().fold(0, |sum, i| match i {
                            Lisp::Num(i) => sum + i,
                            _ => panic!("Can't add non-numbers"),
                        });
                        // Indicate that we want to return a value.
                        return Ok(FrameStepResult::Return(Lisp::Num(sum)));
                    }
                    _ => panic!("Not operation, or operation not implemented"),
                }
            } else {
                // We're still evaling arguments.
                let l = self.list.remove(0); // TODO: use pop and reverse arg list

                // Indicate to the evaler that we want to recur on the next arg
                // fragment.
                return Ok(FrameStepResult::Recur(l));
            }
        }
    }

    // Make a new frame appropriate for the given lisp fragment. Boxed because
    // it will be a trait object.
    fn match_frame(lisp: Lisp) -> Box<Frame> {
        match lisp {
            Lisp::List(_) => Box::new(ApplicationFrame::new(lisp)),
            x => Box::new(ValueFrame { lisp: x }),
        }
    }

    /// Stepped evaluator for Ironic Space Lisp.
    ///
    /// Normal recursive implementations of lisp evaluators are really simple.
    /// The recursive nature of languages and ASTs, particularly Lisp (see
    /// `::data::Lisp`), make them ideal. However, it's very hard to pause these
    /// evaluators in the middle of their evaluation. Rust in particular doesn't
    /// have continuations, or a way to pause execution. This evaler seeks to
    /// make single step evaluation, limited step evaluation, and continuous
    /// evalutation possible, while gaining stack limits for free. The ultimate
    /// goal is to make preemptive scheduling of Ironic Space Lisp VMs possible.
    #[derive(Debug)]
    pub struct Evaler {
        return_value: Option<Lisp>,
        frames: Vec<Box<Frame>>,
    }

    impl Evaler {
        /// Attempt to evaluate the given lisp fragment.
        ///
        /// Loads the evaler with the given lisp fragment. Once the fragment is
        /// evaluated, the Evaler cannot be reset with a new fragment, and must
        /// be discarded.
        pub fn new(lisp: Lisp) -> Evaler {
            Evaler {
                frames: vec![match_frame(lisp)],
                return_value: None,
            }
        }

        /// Whether the evaler has completely evaluated the lisp fragment.
        pub fn is_done(&self) -> bool {
            self.frames.len() == 0
        }

        /// Step the evaluator until evaluates to a single value.
        ///
        /// If any of the single steps returns an error, it returns it instead
        /// of continuing evaluation. See `Evaler::single_step` for more details.
        ///
        /// Additionally, this prints the evaler state before every step.
        pub fn step_until_return(&mut self) -> Result<Lisp, VmGeneralError> {
            while !self.is_done() {
                println!("{:?}", self);

                self.single_step()?;
            }

            match self.return_value {
                Some(l) => Ok(l.clone()),
                None => Err(VmGeneralError)
            }
        }

        /// Execute the evaler for a single step.
        ///
        /// This function will return an error if it encounters one. If you
        /// continue to step the evaler after it returns an error, its behavior
        /// is undefined, in the sense that I've never tried it and have no idea
        /// what will happen, because the single step function can return errors
        /// from a number of different places, and could potentially leave the
        /// evaler in an invalid state.
        pub fn single_step(&mut self) -> Result<(), VmGeneralError> {
            // Borrow hacks
            let mut pop_frame = false;
            let mut new_frame = None;

            {
                let frame = self.frames.last_mut().unwrap();

                let fsr = frame.single_step(&mut self.return_value)?;

                match fsr {
                    // Do nothing to the control flow.
                    FrameStepResult::Continue => (),
                    FrameStepResult::Return(l) => {
                        self.return_value = Some(l);
                        pop_frame = true;
                    }
                    // Note that this pushes the next frame to the stack.
                    // Currently can only "self-recur".
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
            Ok(()) // TODO: maybe this should return something?
        }
    }
}
