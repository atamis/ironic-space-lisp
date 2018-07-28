use std::mem;
use std::rc::Rc;

use data::Lisp;
use data::Op;
use errors::*;

use super::Frame;
use super::FrameStepResult;

// Will represent function application, currently handles operations.
#[derive(Debug)]
pub struct ApplicationFrame {
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
            _ => panic!("Attempted to make ApplicationFrame with lisp that wasn't an application"),
        }
    }
}

impl Frame for ApplicationFrame {
    // This function is basically backwards.
    fn single_step(&mut self, return_val: &mut Option<Lisp>) -> Result<FrameStepResult> {
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
                    // TODO: Hack
                    let mut encountered_nonnumber = false;
                    // Sum everything up, and aggressively panic if we can't
                    // add correctly.
                    let sum = args.iter().fold(0, |sum, i| match i {
                        Lisp::Num(i) => sum + i,
                        _ => {
                            encountered_nonnumber = true;
                            sum
                        }
                    });
                    if encountered_nonnumber {
                        return Err("Attempted to add non-number".into());
                    }
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
