use std::rc::Rc;

use data::Lisp;
use errors::*;

use super::Frame;
use super::FrameStepResult;

// Represents a single value fragment, usually a data literal.
#[derive(Debug)]
pub struct ValueFrame {
    lisp: Rc<Lisp>,
}

impl ValueFrame {
    pub fn new(lisp: Rc<Lisp>) -> ValueFrame {
        ValueFrame { lisp }
    }
}

impl Frame for ValueFrame {
    fn single_step(&mut self, _return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult> {
        Ok(
            FrameStepResult::Return(
                Rc::clone( &self.lisp )
            )
        )
    }

    fn is_appropriate(_lisp: Rc<Lisp>) -> bool where Self: Sized {
        true
    }
}
