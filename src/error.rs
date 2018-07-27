use std::error;
use std::fmt;
use std::convert::From;

#[derive(Debug)]
pub enum VmError {
    General(VmGeneralError),
    Pop(VmPopError),
    Type(VmTypeError),
}
impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //write!(f, "Ironic Space Lisp VM encountered a runtime error")
        match self {
            VmError::General(e) => e.fmt(f),
            VmError::Pop(e) => e.fmt(f),
            VmError::Type(e) => e.fmt(f),
        }
    }
}

impl error::Error for VmError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl From<VmPopError> for VmError {
    fn from(error: VmPopError) -> Self {
        VmError::Pop(error)
    }
}

impl From<VmGeneralError> for VmError {
    fn from(error: VmGeneralError) -> Self {
        VmError::General(error)
    }
}
impl From<VmTypeError> for VmError {
    fn from(error: VmTypeError) -> Self {
        VmError::Type(error)
    }
}

#[derive(Debug)]
pub struct VmGeneralError;

impl fmt::Display for VmGeneralError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ironic Space Lisp VM encountered a runtime error")
    }
}

impl error::Error for VmGeneralError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(Debug)]
pub struct VmTypeError(pub String, pub String);

impl fmt::Display for VmTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Type error: expected {}, got {}", self.0, self.1)
    }
}

impl error::Error for VmTypeError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}


#[derive(Debug)]
pub struct VmPopError;

impl fmt::Display for VmPopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Attempted to pop empty VM stack.")
    }
}

impl error::Error for VmPopError {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}
