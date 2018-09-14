#[allow(clippy)]
pub mod isl;

use data;
use data::Literal;
use errors::*;
use failure::Error;
use lalrpop_util;
use std::fmt::Debug;
use std::fmt::Display;
use nom::types::CompleteStr;
use nom::{ digit, anychar };
use std::str::FromStr;

pub struct Parser(isl::ExprsParser);

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}


impl Parser {
    pub fn new() -> Parser {
        Parser(isl::ExprsParser::new())
    }

    pub fn parse(&self, input: &str) -> Result<Vec<data::Literal>> {
        self.0.parse(input).map_err(|e| Parser::wrap_err(&e))
    }

    fn wrap_err<A, B, C>(e: &lalrpop_util::ParseError<A, B, C>) -> Error
    where
        A: Display + Debug,
        B: Display + Debug,
        C: Display + Debug,
    {
        format_err!("ParseError: {:?}", e)
    }
}

// These get used in macros, but rust doesn't recognize that
#[allow(dead_code)]
fn keyword_element_first(s: char) -> bool {
    s.is_alphabetic() ||
        "-!??*+/$<>.=".contains(s)
}
#[allow(dead_code)]
fn keyword_element(s: char) -> bool {
    keyword_element_first(s) || s.is_numeric()
}

named!(number<CompleteStr, Literal >, map!(digit, |s| Literal::Number(u32::from_str(&s).unwrap())));

named!(keyword<CompleteStr, Literal >,
       do_parse!(
           f: verify!(anychar, keyword_element_first) >>
               rest: take_while!(keyword_element) >>
               (Literal::Keyword({
                   let mut s = rest.to_string();
                   s.insert(0, f);
                   s
               }))
       )
);

named!(expr<CompleteStr, Literal >,
       alt!(keyword | number |
            map!(delimited!(tag!("("),
                            exprs,
                            tag!(")")),
                 |v| data::list(v))
    )
);

named!(tagged_expr<CompleteStr, Literal>,
      do_parse!(
          tag: opt!(alt!(tag!("'") | tag!("`") | tag!(","))) >>
              expr: expr >>
              ({
                  match tag {
                      Some(s) => {
                          let key = match s.to_string().as_ref() {
                              "'" => "quote",
                              "`" => "quasiquote",
                              "," => "unquote",
                              _ => unreachable!(),
                          };

                          data::list(vec![Literal::Keyword(key.to_string()), expr])
                      },
                      None => expr,
                  }
              })
      )
);

named!(exprs<CompleteStr, Vec<Literal> >, many0!(complete!(ws!(tagged_expr))));

#[cfg(test)]
mod tests {
    use super::*;
    use nom::IResult;

    fn cstr(s: &str) -> CompleteStr {
        CompleteStr(s)
    }

    fn unwr<T, L>(r: IResult<T, L>) -> Result<L> {
        match r {
            Ok((_, o)) => Ok(o),
            Err(_) => Err(err_msg("Parse error or something")),
        }
    }


    fn apper<F, T>(f: F) -> Box<Fn(&str) -> Result<T>>
    where F: Fn(CompleteStr) -> IResult<CompleteStr, T> + 'static {
        Box::new(move |s: &str| unwr(f(cstr(s))))
    }

    fn app<F, T>(f: F, s: &str) -> Result<T> where F: Fn(CompleteStr) -> IResult<CompleteStr, T> {
        unwr(f(cstr(s)))
    }

    use data::list;
    use data::Literal;
    use data::Literal::Keyword;
    use data::Literal::Number;
    use parser::isl;

    fn k(s: &str) -> Literal {
        Keyword(s.to_string())
    }

    #[test]
    fn isl_test_num() {
        let p = apper(number);
        assert!(p("22").is_ok());
        assert_eq!(p("22").unwrap(), Number(22));


        assert_eq!(p("304032").unwrap(), Number(304032));
    }

    #[test]
    fn isl_test_keyword() {
        let p = apper(keyword);

        assert_eq!(p("asdf").unwrap(), k("asdf"));
        assert_eq!(p("a1234").unwrap(), k("a1234"));
        assert_eq!(p("a12-34").unwrap(), k("a12-34"));

        assert_eq!(p("+").unwrap(), k("+"));
        assert_eq!(p("-").unwrap(), k("-"));
        assert_eq!(p("*").unwrap(), k("*"));
        assert_eq!(p("/").unwrap(), k("/"));

        assert_eq!(p("asdf?").unwrap(), k("asdf?"));
        assert_eq!(p("asdf!").unwrap(), k("asdf!"));
        assert_eq!(p("<>").unwrap(), k("<>"));
        assert_eq!(p("><").unwrap(), k("><"));

        assert_eq!(p("asdf.qwer").unwrap(), k("asdf.qwer"));

        assert!(p("1234").is_err())
    }

    #[test]
    fn isl_test_items() {
        let k = |s: &str| Keyword(s.to_string());

        let p = apper(exprs);

        assert_eq!(p("asdf").unwrap(), vec![k("asdf")]);
        assert_eq!(p("asdf qwer").unwrap(), vec![k("asdf"), k("qwer")]);

        assert_eq!(p("1234").unwrap(), vec![Number(1234)]);
        assert_eq!(
            p("1234 5678").unwrap(),
            vec![Number(1234), Number(5678)]
        );

        assert_eq!(
            p("1234 asdf\n qwer").unwrap(),
            vec![Number(1234), k("asdf"), k("qwer")]
        );
    }

    #[test]
    fn isl_test_list() {
        let k = |s: &str| Keyword(s.to_string());

        let p = apper(expr);

        assert_eq!(p("()").unwrap(), list(vec![]));
        assert_eq!(p("(asdf)").unwrap(), list(vec![k("asdf")]));
        assert_eq!(p("(  asdf   )").unwrap(), list(vec![k("asdf")]));
        assert_eq!(
            p("(  asdf  1234 )").unwrap(),
            list(vec![k("asdf"), Number(1234)])
        );

        assert!(p("(").is_err());
        assert!(p(")").is_err());
    }

    #[test]
    fn isl_test_nested_exprs() {
        let k = |s: &str| Keyword(s.to_string());

        let p = apper(expr);

        assert_eq!(
            p("(((())))").unwrap(),
            list(vec![list(vec![list(vec![list(vec![])])])])
        );

        assert_eq!(
            p("(test1 (+ 1 2 3 4))").unwrap(),
            list(vec![
                k("test1"),
                list(vec![k("+"), Number(1), Number(2), Number(3), Number(4)])
            ])
        );
    }

    #[test]
    fn isl_test_quotes() {
        let p = apper(tagged_expr);

        assert_eq!(
            p("'(1 2 3 4)").unwrap(),
            list(vec![
                k("quote"),
                list(vec![Number(1), Number(2), Number(3), Number(4)])
            ])
        );

        assert_eq!(
            p("`(1 2 3 4)").unwrap(),
            list(vec![
                k("quasiquote"),
                list(vec![Number(1), Number(2), Number(3), Number(4)])
            ])
        );

        assert_eq!(
            p(",(1 2 3 4)").unwrap(),
            list(vec![
                k("unquote"),
                list(vec![Number(1), Number(2), Number(3), Number(4)])
            ])
        );
    }
}
