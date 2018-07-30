use std::convert::From;
#[allow(unused_attributes)]
#[macro_use]
#[allow(unused_imports)]
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
