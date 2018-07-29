#[macro_use]
extern crate error_chain;

/// Contains global data definitions for Ironic Space Lisp.
///
/// These data structures are used to represent both code/AST and live data.
/// See `::vm` for evaluators using these structures.
pub mod data;

/// Contains out of date error types and a lot of boilerplate.
pub mod errors;

/// Stack frame code.
mod frames;

/// Ironic Space Lisp VM.
///
/// Contains evaluator for Ironic Space Lisp. Utilizes data structures from
/// `::data` and errors from `::error`.
pub mod vm {

    use std::rc::Rc;
    use data::Lisp;
    use errors::*;
    use frames::*;

    /// Normal recursive eval for lisp.
    ///
    /// This is more of an example of just how easy this stuff is to write in a
    /// normal recursive mode. Panics aggressively.
    /*#[deprecated]
    pub fn normal_eval(l: Lisp) -> Lisp {
        match l {
            Lisp::List(rc) => {
                // Main recursion here:
                let l: Vec<Lisp> = (*rc ).iter().map(|r| { normal_eval(r.try_unwrap()) }).collect();
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
            x => x,
        }
    }*/

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
        lisp: Rc<Lisp>,
        return_value: Option<Rc<Lisp>>,
        frames: Vec<Box<Frame>>,
    }

    impl Evaler {
        /// Attempt to evaluate the given lisp fragment.
        ///
        /// Loads the evaler with the given lisp fragment. Once the fragment is
        /// evaluated, the Evaler cannot be reset with a new fragment, and must
        /// be discarded.
        pub fn new(lisp: Lisp) -> Result<Evaler> {
            let r = Rc::new(lisp);
            let r_frame = Rc::clone(&r);
            Ok(
                Evaler {
                    lisp: r,
                    frames: vec![
                        match_frame(r_frame).chain_err(|| "finding suitable frame with match_frame")?,
                    ],
                    return_value: None,
                }
            )
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
        pub fn step_until_return(&mut self) -> Result<Rc<Lisp>> {
            while !self.is_done() {
                println!("{:?}", self);

                self.single_step()
                    .chain_err(|| "Continuously stepping until done")?;
            }

            match self.return_value {
                Some(ref l) => Ok(l.clone()),
                None => Err("No return error found".into()),
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
        pub fn single_step(&mut self) -> Result<()> {
            // Borrow hacks
            let mut pop_frame = false;
            let mut new_frame = None;

            {
                let frame = self.frames.last_mut().ok_or("No frames left")?;

                let fsr = frame
                    .single_step(&mut self.return_value)
                    .chain_err(|| "Executing current frame's single step.")?;

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
                        new_frame = Some(match_frame(l).chain_err(|| "finding suitable frame for recursion")?)
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
