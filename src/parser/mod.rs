#[allow(clippy)]
pub mod isl;

use data;
use errors::*;


pub struct Parser(isl::ExprsParser);


impl Parser {
    pub fn new() -> Parser {
        Parser(isl::ExprsParser::new())
    }

    pub fn parse(&self, input: &str) -> Result<Vec<data::Literal>> {
        match self.0.parse(input) {
            Ok(x) => Ok(x),
            // TODO: not this
            Err(_) => Err(err_msg("Parse error")),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use parser::isl;
    use data::Literal;
    use data::Literal::Number;
    use data::Literal::Keyword;
    use data::list;

    fn k(s: &str) -> Literal {
        Keyword(s.to_string())
    }

    #[test]
    fn isl_test_num() {
        let p = isl::NumParser::new();
        assert!(p.parse("22").is_ok());
        assert_eq!(p.parse("22").unwrap(), Number(22));

        assert_eq!(p.parse("304032").unwrap(), Number(304032));
    }

    #[test]
    fn isl_test_keyword() {
        let p = isl::KeywordParser::new();

        assert_eq!(p.parse("asdf").unwrap(), k("asdf"));
        assert_eq!(p.parse("a1234").unwrap(), k("a1234"));
        assert_eq!(p.parse("a12-34").unwrap(), k("a12-34"));

        assert_eq!(p.parse("+").unwrap(), k("+"));
        assert_eq!(p.parse("-").unwrap(), k("-"));
        assert_eq!(p.parse("*").unwrap(), k("*"));
        assert_eq!(p.parse("/").unwrap(), k("/"));

        assert_eq!(p.parse("asdf?").unwrap(), k("asdf?"));
        assert_eq!(p.parse("asdf!").unwrap(), k("asdf!"));
        assert_eq!(p.parse("<>").unwrap(), k("<>"));
        assert_eq!(p.parse("><").unwrap(), k("><"));

        assert_eq!(p.parse("asdf.qwer").unwrap(), k("asdf.qwer"));

        assert!(p.parse("1234").is_err())
    }

    #[test]
    fn isl_test_items() {
        let k = |s: &str| Keyword(s.to_string());

        let p = isl::ExprsParser::new();

        assert_eq!(p.parse("asdf").unwrap(), vec![k("asdf")]);
        assert_eq!(p.parse("asdf qwer").unwrap(), vec![k("asdf"), k("qwer")]);

        assert_eq!(p.parse("1234").unwrap(), vec![Number(1234)]);
        assert_eq!(p.parse("1234 5678").unwrap(), vec![Number(1234), Number(5678)]);



        assert_eq!(p.parse("1234 asdf\n qwer").unwrap(), vec![Number(1234), k("asdf"), k("qwer")]);
    }

    #[test]
    fn isl_test_list() {
        let k = |s: &str| Keyword(s.to_string());

        let p = isl::ListParser::new();

        assert_eq!(p.parse("()").unwrap(), list(vec![]));
        assert_eq!(p.parse("(asdf)").unwrap(), list(vec![k("asdf")]));
        assert_eq!(p.parse("(  asdf   )").unwrap(), list(vec![k("asdf")]));
        assert_eq!(p.parse("(  asdf  1234 )").unwrap(), list(vec![k("asdf"), Number(1234)]));

        assert!(p.parse("(").is_err());
        assert!(p.parse(")").is_err());
    }

    #[test]
    fn isl_test_nested_exprs() {
        let k = |s: &str| Keyword(s.to_string());

        let p = isl::ExprParser::new();

        assert_eq!(p.parse("(((())))").unwrap(),
                   list(vec![list(vec![list(vec![list(vec![])])])]));

        assert_eq!(p.parse("(test1 (+ 1 2 3 4))").unwrap(),
                   list(vec![k("test1"), list(vec![k("+"), Number(1), Number(2), Number(3), Number(4)])])
        );
    }
}
