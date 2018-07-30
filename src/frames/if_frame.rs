use std::rc::Rc;

use data::Lisp;
use data::Op;
use errors::*;

use super::Frame;
use super::FrameStepResult;

#[derive(Debug)]
pub struct IfFrame {
    lisp: Rc<Lisp>,
    predicate: Option<Rc<Lisp>>,
    answer: Option<Rc<Lisp>>,
    state: FState,
}

impl IfFrame {
    pub fn new(lisp: Rc<Lisp>) -> IfFrame {
        IfFrame {
            lisp,
            predicate: None,
            answer: None,
            state: FState::Starting,
        }
    }
}

#[derive(Debug)]
enum FState {
    Starting,
    Predicate,
    Answer,
}

impl IfFrame {
    fn starting_state(&mut self, _return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult> {
        let list = if let Lisp::List(ref rc) = *self.lisp {
            rc
        } else {
            return Err("if frame not provided a list".into())
        };

        let predicate_frag = list.get(1).ok_or("No predicate in if-frame")?;

        self.state = FState::Predicate;
        return Ok(FrameStepResult::Recur(Rc::clone(predicate_frag)));
    }

    fn predicate_state(&mut self, return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult> {
        if let Some(l) = return_val {
            self.predicate = Some(Rc::clone(l))
        } else {
            return Err("IfFrame in predicate state, but no return value".into());
        }

        let list = if let Lisp::List(ref rc) = *self.lisp {
            rc
        } else {
            return Err("if frame not provided a list".into())
        };

        let true_arm_frag = list.get(2).ok_or("No true-arm in if-frame")?;
        let false_arm_frag = list.get(3).ok_or("No false-arm in if-frame")?;

        if let Some(ref ans) = self.predicate {
            self.state = FState::Answer;

            if **ans == Lisp::Num(3) {
                return Ok(FrameStepResult::Recur(Rc::clone(true_arm_frag)))
            } else {
                return Ok(FrameStepResult::Recur(Rc::clone(false_arm_frag)))
            }
        } else {
            panic!("This should never happen, because we just set predicate to non-None.");
        }

    }

    fn answer_state(&mut self, return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult> {
        if let Some(l) = return_val {
            return Ok(FrameStepResult::Return(Rc::clone(l)))
        } else {
            return Err("IfFrame in predicate state, but no return value".into());
        }
    }

}

impl Frame for IfFrame {
    fn single_step(&mut self, return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult> {
        return match self.state {
            FState::Starting => self.starting_state(return_val),
            FState::Predicate => self.predicate_state(return_val),
            FState::Answer => self.answer_state(return_val),
        };
    }

    fn is_appropriate(lisp: Rc<Lisp>) -> bool where Self: Sized {
        // Deref RC and destructure to the contained vector
        if let Lisp::List(ref l) = *lisp {
            if l.len() > 0 {
                // Some of these might not be necessary.
                if let Lisp::Op(Op::If) = *(**l)[0] { // Pointers were a mistake
                    return true
                }
            }
        }

        false
    }
}
