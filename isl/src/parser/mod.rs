//! Parser for parsing [`Literal`](data::Literal)s values from strings.
//!
//! The string representation of [`Literal`](data::Literal) is a little inconsistent:
//! The [`Debug`](std::fmt::Debug) implementation has some extra debug information,
//! and this parser can't parse it correctly. The extra information is
//! useful, but not necessary.
//!
//! This parser parses what is basically a SEXPR format that maps to a
//! subset of the [`Literal`](data::Literal) enum. Values are numbers, keywords,
//! and lists, which may contain 1 or more additional values.
//!
//! Numbers are currently unsigned. The parser will error if the number is
//! too large. This outputs a [`Literal::Number`](data::Literal::Number)
//!
//! ```
//! # use isl::parser;
//! # use isl::data::Literal;
//! assert_eq!(parser::parse("123").unwrap()[0], Literal::Number(123));
//! ```
//!
//! Symbols a strings of characters that are alphanumeric, or in the set
//! `"-!??*+/$<>.="`, except for the first character, which cannot be
//! numeric. This outputs a [`Literal::Symbol`](data::Literal::Symbol)
//!
//! ```
//! # use isl::parser::parse;
//! # use isl::data::Literal;
//! parse("asdf");
//! parse("+");
//! parse("a123");
//! parse("<html>");
//! parse("</html>");
//! ```
//!
//! Lists are surrounded by matching parentheses, and output a
//! [`Literal::List`](data::Literal::List), and contain 0 or more other literals. They are not
//! separated by commas.
//!
//! ```
//! # use isl::parser::parse;
//! # use isl::data::Literal;
//! parse("(+ 1 2 3)");
//! parse("(((((())))))");
//! parse("(if (< x 2) () (inc x))");
//! ```
//!
//! This parser also handles quoting, and related "reader macros".
//!
//! ```
//! # use isl::parser::parse;
//! # use isl::data::Literal;
//! assert_eq!(parse("'1").unwrap(),
//!            parse("(quote 1)").unwrap());
//!
//! assert_eq!(parse("'keyword").unwrap(),
//!            parse("(quote keyword)").unwrap());
//!
//! assert_eq!(parse("`keyword").unwrap(),
//!            parse("(quasiquote keyword)").unwrap());
//!
//! assert_eq!(parse("`(+ 1 2 ~x)").unwrap(),
//!            parse("(quasiquote (+ 1 2 (unquote x)))").unwrap());
//! ```
//!
//! Note that [`parser::parse`](parse) attempts to parse the string completely
//! into potentially multiple literal values, which it returns as an vector.
//! However, the parser exposes the raw nom parsers `exprs`, `tagged_expr`,
//! and `expr`, which could be used to parse single literals.
//!
//! This parser uses `nom::types::CompleteStr`, which ensures the input
//! strings are completely consumed.
use crate::data;
use crate::data::Literal;
use crate::errors::*;

/// Legacy struct, delegates to [`parser::parse`](parse)
pub struct Parser();

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Create a new parser.
    pub fn new() -> Parser {
        Parser()
    }

    /// Delegates to `parser::parse()`
    pub fn parse(&self, input: &str) -> Result<Vec<data::Literal>> {
        parse(input)
    }
}

/// Parses a string to a vector of [`Literal`](data::Literal)s.
// pub fn parse(input: &str) -> Result<Vec<data::Literal>> {
//     let mut input = CompleteStr(input);
//     let mut lits = vec![];

//     while input != CompleteStr("") {
//         match tagged_expr(input) {
//             Ok((rem, l)) => {
//                 lits.push(l);
//                 input = rem;
//             }
//             e => return Err(format_err!("Parse error: {:?}", e)),
//         }
//     }

//     Ok(lits)
// }

pub fn parse(input: &str) -> Result<Vec<data::Literal>> {
    Ok(read_all(input)?.iter().map(Literal::from).collect())
}

fn read_all(input: &str) -> Result<Vec<edn::Value>> {
    let mut parser = edn::parser::Parser::new(input);
    let mut out = vec![];

    while let Some(r) = parser.read() {
        out.push(r.map_err(|e| format_err!("Parser error [{}:{}]: {}", e.lo, e.hi, e.message))?);
    }

    Ok(out)
}

impl From<&edn::Value> for Literal {
    fn from(v: &edn::Value) -> Literal {
        use edn::Value;

        match v {
            Value::Nil => Literal::Nil,
            Value::Boolean(b) => (*b).into(),
            Value::String(s) => Literal::String(s.into()),
            Value::Char(c) => Literal::Char(*c),
            Value::Integer(n) => Literal::Number(*n),
            Value::Float(n) => Literal::Float(*n),
            Value::Symbol(s) => Literal::Symbol(s.to_string()),
            Value::Keyword(s) => Literal::Keyword(s.to_string()),
            Value::List(v) => Literal::List(v.iter().map(|x| x.into()).collect::<im::Vector<_>>()),
            Value::Vector(v) => {
                Literal::Vector(v.iter().map(|x| x.into()).collect::<im::Vector<_>>())
            }
            Value::Map(v) => Literal::Map(
                v.iter()
                    .map(|(k, v)| -> (Literal, Literal) { (k.into(), v.into()) })
                    .collect::<im::OrdMap<_, _>>(),
            ),
            Value::Set(s) => Literal::Set(
                s.iter()
                    .map(|x| -> Literal { x.into() })
                    .collect::<im::OrdSet<_>>(),
            ),
            Value::Tagged(s, b) => Literal::Tagged(s.to_string(), Box::new((&**b).into())),
            //_ => panic!(format!("Not implemented: {:?}", v)),
        }
    }
}

/*fn cstr(s: &str) -> CompleteStr {
    CompleteStr(s)
}

fn unwr<T, L>(r: IResult<T, L>) -> Result<L>
where
    T: Debug,
{
    match r {
        Ok((_, o)) => Ok(o),
        Err(e) => Err(format_err!("Parse error: {:?}", e)),
    }
}

/// Applies a parser function to a string to get a value.
///
/// Parsers take a special form of string, and produces its own result type.
/// This function wraps the string, and unwraps the result, and repackages
/// it into the ISL's Result type.
pub fn app<F, T>(f: F, s: &str) -> Result<T>
where
    F: Fn(CompleteStr) -> IResult<CompleteStr, T>,
{
    unwr(f(cstr(s)))
}

/// Wraps a parser function to make it easier to use.
///
/// The wrapper function wraps and unwraps input to the function. See [`app`](app) for more info.
pub fn apper<F, T>(f: F) -> Box<dyn Fn(&str) -> Result<T>>
where
    F: Fn(CompleteStr) -> IResult<CompleteStr, T> + 'static,
{
    Box::new(move |s: &str| unwr(f(cstr(s))))
}

// These get used in macros, but rust doesn't recognize that
#[allow(dead_code)]
fn keyword_element_first(s: char) -> bool {
    s.is_alphabetic() || "-!??*+/$<>.=".contains(s)
}
#[allow(dead_code)]
fn keyword_element(s: char) -> bool {
    keyword_element_first(s) || s.is_numeric()
}

named!(number<CompleteStr, Literal >, map_res!(digit, |s: CompleteStr| u32::from_str(&s).map(Literal::Number)));

named!(keyword<CompleteStr, Literal >,
       do_parse!(
           f: verify!(anychar, keyword_element_first) >>
               rest: take_while!(keyword_element) >>
               (Literal::Symbol({
                   let mut s = rest.to_string();
                   s.insert(0, f);
                   s
               }))
       )
);

named!(boolean<CompleteStr, Literal>,
       alt!(
           value!(Literal::Boolean(true), tag!("#t")) |
           value!(Literal::Boolean(false), tag!("#f"))
           //value!(Literal::Boolean(true), tag!("true")) |
           //value!(Literal::Boolean(true), tag!("false"))
       )
);

named_attr!(#[doc = "Raw nom parser for parsing a single untagged expr."], pub expr<CompleteStr, Literal >,
       alt!(keyword | number | boolean |
            map!(alt!(
                delimited!(tag!("("),
                           exprs,
                           tag!(")")) |
                delimited!(tag!("["),
                           exprs,
                           tag!("]"))
            ),
                 data::list)
    )
);

named_attr!(#[doc = "Raw nom parser for parsing single tagged exprs."], pub tagged_expr<CompleteStr, Literal>,
            ws!(
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

                          data::list(vec![Literal::Symbol(key.to_string()), expr])
                      },
                      None => expr,
                  }
              })
      )
                    )
);

named_attr!(
    #[doc = "Raw nom parser for parsing mulitple exprs."],
    pub exprs<CompleteStr, Vec<Literal> >, complete!(ws!(many0!(complete!(tagged_expr)))));
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::list;
    use crate::data::Literal;
    use crate::data::Literal::Map;
    use crate::data::Literal::Number;
    use crate::data::Literal::Set;
    use crate::data::Literal::Symbol;
    use crate::data::Literal::Vector;

    fn k(s: &str) -> Literal {
        Symbol(s.to_string())
    }

    fn p(s: &str) -> Result<Vec<Literal>> {
        parse(s)
    }

    fn p1(s: &str) -> Result<Literal> {
        Ok(p(s)?.pop().unwrap())
    }

    #[test]
    fn isl_test_num() {
        assert!(p1("22").is_ok());
        assert_eq!(p1("22").unwrap(), Number(22));

        assert_eq!(p1("304032").unwrap(), Number(304032));
        assert!(p1("99999999999999999999999999999999999999999999").is_err());
    }

    #[test]
    fn isl_test_keyword() {
        assert_eq!(p1("asdf").unwrap(), k("asdf"));
        assert_eq!(p1("a1234").unwrap(), k("a1234"));
        assert_eq!(p1("a12-34").unwrap(), k("a12-34"));

        assert_eq!(p1("+").unwrap(), k("+"));
        assert_eq!(p1("-").unwrap(), k("-"));
        assert_eq!(p1("*").unwrap(), k("*"));
        assert_eq!(p1("/").unwrap(), k("/"));

        assert_eq!(p1("asdf?").unwrap(), k("asdf?"));
        assert_eq!(p1("asdf!").unwrap(), k("asdf!"));
        assert_eq!(p1("<>").unwrap(), k("<>"));
        assert_eq!(p1("><").unwrap(), k("><"));

        assert_eq!(p1("asdf.qwer").unwrap(), k("asdf.qwer"));

        // Because we can't isolate tests anymore, this will have to do
        //assert!(p1("1234").is_err())
    }

    #[test]
    fn isl_test_boolean() {
        assert_eq!(p1("true").unwrap(), Literal::Boolean(true));
        assert_eq!(p1("false").unwrap(), Literal::Boolean(false));

        assert_eq!(p1("( true )").unwrap(), list(vec![Literal::Boolean(true)]));
    }

    #[test]
    fn isl_test_items() {
        let k = |s: &str| Symbol(s.to_string());

        assert_eq!(p("asdf").unwrap(), vec![k("asdf")]);
        assert_eq!(p("asdf qwer").unwrap(), vec![k("asdf"), k("qwer")]);

        assert_eq!(p("1234").unwrap(), vec![Number(1234)]);
        assert_eq!(p("1234 5678").unwrap(), vec![Number(1234), Number(5678)]);

        assert_eq!(
            p("1234 asdf\n qwer").unwrap(),
            vec![Number(1234), k("asdf"), k("qwer")]
        );
    }

    #[test]
    fn isl_test_list() {
        let k = |s: &str| Symbol(s.to_string());

        assert_eq!(p1("()").unwrap(), list(vec![]));
        assert_eq!(p1("(asdf)").unwrap(), list(vec![k("asdf")]));
        assert_eq!(p1("(  asdf   )").unwrap(), list(vec![k("asdf")]));
        assert_eq!(
            p1("(  asdf  1234 )").unwrap(),
            list(vec![k("asdf"), Number(1234)])
        );

        assert!(p("(").is_err());
        assert!(p(")").is_err());
    }

    #[test]
    fn isl_test_nested_exprs() {
        let k = |s: &str| Symbol(s.to_string());

        assert_eq!(
            p1("(((())))").unwrap(),
            list(vec![list(vec![list(vec![list(vec![])])])])
        );

        assert_eq!(
            p1("(test1 (+ 1 2 3 4))").unwrap(),
            list(vec![
                k("test1"),
                list(vec![k("+"), Number(1), Number(2), Number(3), Number(4)])
            ])
        );
    }

    #[test]
    fn isl_test_quotes() {
        assert_eq!(
            p1("'(1 2 3 4)").unwrap(),
            list(vec![
                k("quote"),
                list(vec![Number(1), Number(2), Number(3), Number(4)])
            ])
        );

        assert_eq!(
            p1("`(1 2 3 4)").unwrap(),
            list(vec![
                k("quasiquote"),
                list(vec![Number(1), Number(2), Number(3), Number(4)])
            ])
        );

        assert_eq!(
            p1("~(1 2 3 4)").unwrap(),
            list(vec![
                k("unquote"),
                list(vec![Number(1), Number(2), Number(3), Number(4)])
            ])
        );
    }

    #[test]
    fn test_vector() {
        assert_eq!(p1("[]").unwrap(), Vector(vector![]));
        assert_eq!(
            p1("[1 2 3]").unwrap(),
            Vector(vector![Number(1), Number(2), Number(3)])
        );
    }

    #[test]
    fn test_map() {
        assert_eq!(p1("{}").unwrap(), Map(ordmap! {}));
        assert_eq!(
            p1("{1 2 3 4}").unwrap(),
            Map(ordmap! {Number(1) => Number(2), Number(3) => Number(4)})
        );
        assert!(p1("{1 2 3}").is_err());
        assert!(p1("{1}").is_err());
    }

    #[test]
    fn test_set() {
        assert_eq!(p1("#{}").unwrap(), Set(ordset![]));
        assert_eq!(
            p1("#{1 2 3 4}").unwrap(),
            Set(ordset![Number(1), Number(2), Number(3), Number(4)])
        );

        // TODO
        //assert!(p1("#{1 1}").is_err());
    }

    // It's assumed these are tested more exhaustively by the external parser.
    #[test]
    fn test_nil() {
        assert_eq!(p1("nil").unwrap(), Literal::Nil);
    }

    #[test]
    fn test_string() {
        assert_eq!(p1("\"test\"").unwrap(), Literal::String("test".into()));
    }

    #[test]
    fn test_char() {
        assert_eq!(p1("\\c").unwrap(), Literal::Char('c'));
    }

    #[test]
    fn test_keyword() {
        assert_eq!(p1(":test").unwrap(), Literal::Keyword("test".into()));
    }

    #[test]
    fn test_float() {
        assert_eq!(p1("1.1").unwrap(), Literal::Float((1.1).into()));
        assert_eq!(p1("+1.1").unwrap(), Literal::Float((1.1).into()));
        assert_eq!(p1("-1.1").unwrap(), Literal::Float((-1.1).into()));
    }

    #[test]
    fn test_tagged() {
        assert_eq!(
            p1("#test true").unwrap(),
            Literal::Tagged("test".into(), Box::new(Literal::Boolean(true)))
        );
    }
}
