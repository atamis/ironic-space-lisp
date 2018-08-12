use std::rc::Rc;

use nom;


/*#[macro_use]
use error_chain;
use std::fmt;

error_chain! {
    errors {
        NomError(desc: String)
    }
}


impl<E: fmt::Debug + Clone> From<nom::Err<E>> for NomError {
    fn from(error: nom::Err<E>) -> Self {
        let desc = match error {
            nom::Err::Incomplete(needed) => format!("ran out of bytes: {:?}", needed),
            nom::Err::Error(context) => format!("{:?}", nom::error_to_list(&context)),
            nom::Err::Failure(context) => format!("{:?}", nom::error_to_list(&context)),
        };

        NomError { desc }
    }
}

impl<E> From<nom::Err<E>> for NomError {
    let desc = match error {
        nom::Err::Incomplete(needed) => format!("ran out of bytes: {:?}", needed),
        _ => "Parse error",
    };

    NomError { desc }
}*/


#[derive(Debug)]
pub enum Expr {
    String(String),
    List(Rc<Vec<Expr>>),
}

#[derive(Debug)]
pub enum Token {
    Open,
    Close,
    Keyword(String),
    Number(u32),
    Quote,
    Quasiquote,
    Unquote,
}

named!(pub open_delim<&str, Token>, value!(Token::Open, tag!("(")));
named!(pub close_delim<&str, Token>, value!(Token::Close, tag!(")")));
//named!(pub keyword<&str, Token>, map!(many_till!(alpha1, is_not!(alpha1)), |(s, _)| Token::Keyword(s.to_string())));
//named!(pub keyword<&str, Token>, map!(alpha1, |s| Token::Keyword(s.to_string())));
named!(pub keyword<&str, Token>, map!(take_while!( char::is_alphabetic ), |s| Token::Keyword(s.to_string())));


named!(pub token<&str, Token>,
       ws!(alt!(
           open_delim |
           close_delim |
           keyword
       ))
);

pub fn tokenize(s: &str) -> Result<Vec<Token>, nom::Err<&str>> {
    tokens(s)
        .and_then(|(_, v)| Ok(v))
}

named!(pub tokens<&str, Vec<Token>>, many0!(complete!(token)));

