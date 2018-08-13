pub mod ast;
pub mod isl;


#[cfg(test)]
mod tests {
    use parser::isl;
    use parser::ast::Lisp;
    use parser::ast::Lisp::Num;
    use parser::ast::Lisp::Keyword;
    use parser::ast::list;

    fn k(s: &str) -> Lisp {
        Keyword(s.to_string())
    }

    #[test]
    fn isl_test_num() {
        let p = isl::NumParser::new();
        assert!(p.parse("22").is_ok());
        assert_eq!(p.parse("22").unwrap(), Num(22));

        assert_eq!(p.parse("304032").unwrap(), Num(304032));
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

        assert_eq!(p.parse("1234").unwrap(), vec![Num(1234)]);
        assert_eq!(p.parse("1234 5678").unwrap(), vec![Num(1234), Num(5678)]);



        assert_eq!(p.parse("1234 asdf\n qwer").unwrap(), vec![Num(1234), k("asdf"), k("qwer")]);
    }

    #[test]
    fn isl_test_list() {
        let k = |s: &str| Keyword(s.to_string());

        let p = isl::ListParser::new();

        assert_eq!(p.parse("()").unwrap(), list(vec![]));
        assert_eq!(p.parse("(asdf)").unwrap(), list(vec![k("asdf")]));
        assert_eq!(p.parse("(  asdf   )").unwrap(), list(vec![k("asdf")]));
        assert_eq!(p.parse("(  asdf  1234 )").unwrap(), list(vec![k("asdf"), Num(1234)]));

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
                   list(vec![k("test1"), list(vec![k("+"), Num(1), Num(2), Num(3), Num(4)])])
        );
    }
}
