use data::Lisp;
use errors::*;

use super::Frame;
use super::FrameStepResult;

// Represents a single value fragment, usually a data literal.
#[derive(Debug)]
pub struct ValueFrame {
    lisp: Lisp,
}

impl ValueFrame {
    pub fn new(lisp: Lisp) -> ValueFrame {
        ValueFrame {
            lisp,
        }
    }
}

impl Frame for ValueFrame {
    // Currently handles numbers and ops.
    fn single_step(
        &mut self,
        _return_val: &mut Option<Lisp>,
    ) -> Result<FrameStepResult> {
        match &self.lisp {
            Lisp::List(_) => Err("Can't make value frame on a list".into()),
            x => Ok(FrameStepResult::Return(x.clone())),
        }
    }
}
