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
    // This is basically a Lisp::List, but we unwrap it for more concise code.
    list: Rc<Vec<Rc<Lisp>>>,
    idx: usize,
    // already evaled args.
    vals: Vec<Rc<Lisp>>,
}

impl ApplicationFrame {
    pub fn new(lisp: Rc<Lisp>) -> ApplicationFrame {
        match *lisp {
            Lisp::List(ref l) => {
                // TODO: maybe don't do this.
                //let list = Rc::try_unwrap(l).unwrap();
                ApplicationFrame {
                    list: Rc::clone(l),
                    idx: 0,
                    vals: Vec::new(),
                }
            }
            _ => panic!("Attempted to make ApplicationFrame with lisp that wasn't an application"),
        }
    }
}

impl Frame for ApplicationFrame {
    // This function is basically backwards.
    fn single_step(&mut self, return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult> {
        // Extract the result of the last fragment we recurred on.
        if let Some(r) = return_val {
            self.vals.push(r.clone());
            //if let Some(myr) = mem::replace(return_val, None) {
                //self.vals.push(&myr);
            //}
        }

        // We've evaled all the arg fragments, so it's time to actually
        // apply the args to the operation.
        if self.list.len() <= self.idx {
            let op = &self.vals[0];
            let args = &self.vals[1..];

            match **op {
                Lisp::Op(Op::Add) => {
                    // TODO: Hack
                    let mut encountered_nonnumber = false;
                    let sum = args.iter().fold(0, |sum, i| match **i {
                        Lisp::Num(n) => sum + n,
                        _ => {
                            encountered_nonnumber = true;
                            sum
                        }
                    });
                    if encountered_nonnumber {
                        return Err("Attempted to add non-number".into());
                    }
                    // Indicate that we want to return a value.
                    return Ok(FrameStepResult::Return(Rc::new(Lisp::Num(sum))));
                },
                _ => panic!("Not operation, or operation not implemented"),
            }
        } else {
            // We're still evaling arguments.
            let l = &self.list[self.idx]; // TODO: use pop and reverse arg list
            self.idx += 1;

            // Indicate to the evaler that we want to recur on the next arg
            // fragment.
            return Ok(FrameStepResult::Recur(Rc::clone(l)));
        }
    }
}
