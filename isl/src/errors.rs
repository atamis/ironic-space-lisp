//! Error handling code
//!
//! Currently reexports some [ `failure` ] types, and `Result<T>` type like `error_chain` used.
#[allow(unused_imports)]
use failure::Error;

pub use failure::err_msg;
pub use failure::ResultExt;

/// Result for user facing errors.
pub type Result<T> = failure::_core::prelude::v1::Result<T, Error>;

/*#[derive(Debug, Fail)]
#[fail(display = "Generic error: {}", _0)]
struct GenericError (String);

impl<'a> From<&'a str> for GenericError {
    fn from(s: &'a str) -> GenericError {
        GenericError(s)
    }
}*/
