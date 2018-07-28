use std::convert::From;
use std::error;
use std::fmt;
#[macro_use]
use error_chain;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    errors {
        VmGeneralError {
            description("General VM error")
            display("General VM error")
        }
    }
}

