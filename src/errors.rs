#[allow(unused_imports)]
use failure::Error;
use failure;

pub use failure::ResultExt;
pub use failure::err_msg;

pub type Result<T> = failure::_core::prelude::v1::Result<T, Error>;

/*#[derive(Debug, Fail)]
#[fail(display = "Generic error: {}", _0)]
struct GenericError (String);

impl<'a> From<&'a str> for GenericError {
    fn from(s: &'a str) -> GenericError {
        GenericError(s)
    }
}*/
