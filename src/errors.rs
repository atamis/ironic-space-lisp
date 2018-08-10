use std::convert::From;
#[allow(unused_attributes)]
#[macro_use]
#[allow(unused_imports)]
use error_chain;

use std::rc;
use data;

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

impl From<rc::Rc<data::Literal>> for Error {
    fn from(f: rc::Rc<data::Literal>) -> Error {
        format!("Error data literal: {:?}", f).into()
    }
}
