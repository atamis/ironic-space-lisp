mod application;
mod value;
mod if_frame;

use std::fmt;
use std::rc::Rc;

use self::application::ApplicationFrame;
use self::value::ValueFrame;
use self::if_frame::IfFrame;
use data::Lisp;
use data::Op;
use errors::*;

// Make a new frame appropriate for the given lisp fragment. Boxed because
// it will be a trait object.
pub fn match_frame(lisp: Rc<Lisp>) -> Result<Box<Frame>> {
    match *lisp {
        Lisp::List(_) => match_frame_list(lisp),
        _ => Ok(Box::new(ValueFrame::new(lisp))),
    }
}

fn match_frame_list(list: Rc<Lisp>) -> Result<Box<Frame>> {
    #[allow(unused_assignments)]
    let mut application = false;
    let mut if_frame = false;

    match *list {
        Lisp::List(ref l) => {
            match *l[0] {
                Lisp::Op(Op::If) => if_frame = true,
                _ => application = true,
            }
        },
        _ => return Err("expected list, got something else".into()),
    }

    if application {
        return Ok(Box::new(ApplicationFrame::new(list)))
    }

    if if_frame {
        return Ok(Box::new(IfFrame::new(list)))
    }

    Err("failed to match frame for frame list".into())
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
}
