mod value;
mod application;

use std::fmt;

use data::Lisp;
use errors::*;
use self::value::ValueFrame;
use self::application::ApplicationFrame;


// Make a new frame appropriate for the given lisp fragment. Boxed because
// it will be a trait object.
pub fn match_frame(lisp: Lisp) -> Box<Frame> {
    match lisp {
        Lisp::List(_) => Box::new(ApplicationFrame::new(lisp)),
        x => Box::new(ValueFrame::new(x)),
    }
}

// Controls flow in evaluation.
#[derive(Debug)]
pub enum FrameStepResult {
    // Don't do anything to the control flow.
    Continue,
    // Indicates that the fragment is done and wants to return a value. Set
    // the return value, and pop the frame stack.
    Return(Lisp),
    // Start to recur on a another piece of lisp code.
    Recur(Lisp),
}

// Requires debug so we can print trait objects.
pub trait Frame: fmt::Debug {
    // Evaluate a single step on the current fragment. Takes a mutable
    // reference to the return val so the fragment can claim the relevant
    // return values from recurring. You can't set the normal return value,
    // you have to return via FrameStepResult.
    fn single_step(
        &mut self,
        return_val: &mut Option<Lisp>,
    ) -> Result<FrameStepResult>;
}

