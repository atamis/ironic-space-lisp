mod application;
mod value;
mod if_frame;

use std::fmt;
use std::rc::Rc;

use self::application::ApplicationFrame;
use self::value::ValueFrame;
use self::if_frame::IfFrame;
use data::Lisp;
use errors::*;

// Make a new frame appropriate for the given lisp fragment. Boxed because
// it will be a trait object.
pub fn match_frame(lisp: Rc<Lisp>) -> Result<Box<Frame>> {

    // Delgate to frames to figure out which is appropriate.
    // Thus there is a struct heirarchy of specificity

    // Forced to clone Rc because borrowing.
    if IfFrame::is_appropriate(Rc::clone(&lisp)) {
        return Ok(Box::new(IfFrame::new(lisp)));
    }

    if ApplicationFrame::is_appropriate(Rc::clone(&lisp)) {
        return Ok(Box::new(ApplicationFrame::new(lisp)));
    }

    // TODO: this isn't a great idea. any unidentifed syntax
    // gets returned as a value.
    if ValueFrame::is_appropriate(Rc::clone(&lisp)) {
        return Ok(Box::new(ValueFrame::new(lisp)));
    }

    Err("Couldn't match code to frame".into())
}

// Controls flow in evaluation.
#[derive(Debug)]
pub enum FrameStepResult {
    // Don't do anything to the control flow.
    Continue,
    // Indicates that the fragment is done and wants to return a value. Set
    // the return value, and pop the frame stack.
    Return(Rc<Lisp>),
    // Start to recur on a another piece of lisp code.
    Recur(Rc<Lisp>),
}

// Requires debug so we can print trait objects.
pub trait Frame: fmt::Debug {
    // Evaluate a single step on the current fragment. Takes a mutable
    // reference to the return val so the fragment can claim the relevant
    // return values from recurring. You can't set the normal return value,
    // you have to return via FrameStepResult.
    fn single_step(&mut self, return_val: &mut Option<Rc<Lisp>>) -> Result<FrameStepResult>;

    // https://doc.rust-lang.org/error-index.html#method-has-no-receiver
    // I have _no_ idea.
    fn is_appropriate(lisp: Rc<Lisp>) -> bool where Self: Sized;
}
