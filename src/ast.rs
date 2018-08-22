use std::rc::Rc;

use data::Keyword;
use data::Literal;
use errors::*;

#[derive(Debug, PartialEq)]
pub struct Def {
    pub name: Keyword,
    pub value: AST,
}

#[derive(Debug, PartialEq)]
pub enum AST {
    Value(Literal),
    If {
        pred: Rc<AST>,
        then: Rc<AST>,
        els: Rc<AST>,
    },
    Def(Rc<Def>),
    Let {
        defs: Vec<Def>,
        body: Rc<AST>,
    },
    Do(Vec<AST>),
    Lambda {
        args: Vec<Keyword>,
        body: Rc<AST>,
    },
    Var(Keyword),
    Application {
        f: Rc<AST>,
        args: Vec<AST>,
    },
}

pub mod passes {

    pub mod unbound {
        use ast::ASTVisitor;
        use ast::Def;
        use ast::AST;
        use data::Keyword;
        use data::Literal;
        use environment::Env;
        use errors::*;
        use im::hashset;
        use std::rc::Rc;

        #[allow(dead_code)]
        type KeywordSet = hashset::HashSet<Keyword>;

        pub fn pass_default(asts: &[AST]) -> Result<()> {
            let mut hs = hashset::HashSet::new();

            asts.iter().map(|a| hs.visit(a)).collect()
        }

        pub fn pass(a: &AST, _env: Env) -> Result<()> {
            // TODO
            let mut hs = hashset::HashSet::new();

            hs.visit(a)
        }

        impl ASTVisitor<()> for KeywordSet {
            fn value_expr(&mut self, _l: &Literal) -> Result<()> {
                Ok(())
            }

            fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<()> {
                self.visit(pred)?;
                self.visit(then)?;
                self.visit(els)?;
                Ok(())
            }

            fn def_expr(&mut self, def: &Rc<Def>) -> Result<()> {
                self.visit(&def.value)?;
                self.insert(def.name.clone());
                Ok(())
            }

            fn let_expr(&mut self, defs: &Vec<Def>, body: &Rc<AST>) -> Result<()> {
                let mut c = self.clone();
                for d in defs {
                    c.insert(d.name.clone());
                }

                c.visit(body)
            }

            fn do_expr(&mut self, exprs: &Vec<AST>) -> Result<()> {
                exprs.iter().map(|e| self.visit(e)).collect()
            }

            fn lambda_expr(&mut self, args: &Vec<Keyword>, body: &Rc<AST>) -> Result<()> {
                let mut c = self.clone();
                for k in args {
                    c.insert(k.clone());
                }

                c.visit(body)
            }

            fn var_expr(&mut self, k: &Keyword) -> Result<()> {
                if self.contains(k) {
                    Ok(())
                } else {
                    Err(format_err!("Unbound var {:}", k))
                }
            }

            fn application_expr(&mut self, f: &Rc<AST>, args: &Vec<AST>) -> Result<()> {
                self.visit(f)?;
                args.iter().map(|e| self.visit(e)).collect()
            }
        }

        #[cfg(test)]
        mod tests {
            use super::pass_default;
            use ast;
            use ast::AST;
            use errors::*;
            use parser;

            fn p(s: &str) -> Result<()> {
                let p = parser::Parser::new();
                let lit = &p.parse(s)?;
                let asts: Vec<AST> = lit.iter().map(ast::parse).collect::<Result<_>>()?;
                pass_default(asts.as_ref())
            }

            #[test]
            fn test_value() {
                assert!(p("24").is_ok());
            }

            #[test]
            fn test_if() {
                assert!(p("(if 0 0 0)").is_ok());
                assert!(p("(if test 0 0)").is_err());
                assert!(p("(if 0 test 0)").is_err());
                assert!(p("(if 0 0 test)").is_err());
            }

            #[test]
            fn test_def() {
                assert!(p("(def test 0)").is_ok());
                assert!(p("(def test asdf)").is_err());
                assert!(p("(def test test)").is_err());

                assert!(p("(def test 0) test").is_ok())
            }

            #[test]
            fn test_let() {
                assert!(p("(let (test 0) asdf)").is_err());
                assert!(p("(let (test 0) test)").is_ok());
            }

            #[test]
            fn test_do() {
                assert!(p("(do)").is_ok());
                assert!(p("(do 1)").is_ok());
                assert!(p("(do test)").is_err());
                assert!(p("(do test 1 1)").is_err());
                assert!(p("(do 1 test 1)").is_err());
                assert!(p("(do 1 1 test)").is_err());
            }

            #[test]
            fn test_lambda() {
                assert!(p("(lambda () 1)").is_ok());
                assert!(p("(lambda () asdf)").is_err());
                assert!(p("(lambda (test) test)").is_ok());
                assert!(p("(lambda (test) asdf)").is_err());
            }

            #[test]
            fn test_single_var() {
                assert!(p("test1").is_err());
            }

            #[test]
            fn test_application() {
                assert!(p("(0)").is_ok());
                assert!(p("(0 0)").is_ok());
                assert!(p("(0 0 0)").is_ok());
                assert!(p("(0 0 0 0)").is_ok());

                assert!(p("(test 0 0 0)").is_err());
                assert!(p("(0 test 0 0)").is_err());
                assert!(p("(0 0 test 0)").is_err());
                assert!(p("(0 0 0 test)").is_err());

                assert!(p("(def test 1)(test 0 0 0)").is_ok());
                assert!(p("(def test 1)(0 test 0 0)").is_ok());
                assert!(p("(def test 1)(0 0 test 0)").is_ok());
                assert!(p("(def test 1)(0 0 0 test)").is_ok());
            }
        }
    }
}

pub trait ASTVisitor<R> {
    fn visit(&mut self, a: &AST) -> Result<R> {
        let r = match a {
            AST::Value(l) => self.value_expr(l).context("Visiting value expr"),
            AST::If { pred, then, els } => {
                self.if_expr(pred, then, els).context("Visiting if expr")
            }
            AST::Def(def) => self.def_expr(def).context("Visiting def expr"),
            AST::Let { defs, body } => self.let_expr(defs, body).context("Fixing let expr"),
            AST::Do(asts) => self.do_expr(asts).context("Visiting do expr"),
            AST::Lambda { args, body } => {
                self.lambda_expr(args, body).context("Visiting lambda expr")
            }
            AST::Var(k) => self.var_expr(k).context("Vising var expr"),
            AST::Application { f, args } => self
                .application_expr(f, args)
                .context("Visiting application expr"),
        }?;

        Ok(r)
    }

    fn value_expr(&mut self, l: &Literal) -> Result<R>;

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<R>;

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<R>;

    fn let_expr(&mut self, defs: &Vec<Def>, body: &Rc<AST>) -> Result<R>;

    fn do_expr(&mut self, exprs: &Vec<AST>) -> Result<R>;

    fn lambda_expr(&mut self, args: &Vec<Keyword>, body: &Rc<AST>) -> Result<R>;

    fn var_expr(&mut self, k: &Keyword) -> Result<R>;

    fn application_expr(&mut self, f: &Rc<AST>, args: &Vec<AST>) -> Result<R>;
}

pub fn parse(e: &Literal) -> Result<AST> {
    match e {
        Literal::List(ref vec) => {
            if let Some((first, rest)) = vec.split_first() {
                parse_compound(first, rest)
            } else {
                Err(err_msg("empty list not valid"))
            }
        }
        Literal::Keyword(k) => Ok(AST::Var(k.clone())),
        Literal::Boolean(_) => Ok(AST::Value(e.clone())),
        Literal::Number(_) => Ok(AST::Value(e.clone())),
        Literal::Address(_) => Err(err_msg("Address literals not supported")),
    }
}

// TODO: break these parsers out into functions and make better error messages.
fn parse_compound(first: &Literal, rest: &[Literal]) -> Result<AST> {
    let r = if let Literal::Keyword(s) = first {
        match s.as_ref() {
            "if" => parse_if(first, rest).context("Parsing let expr"),
            "def" => parse_def_expr(first, rest).context("Parsing def expr"),
            "let" => parse_let(first, rest).context("Parsing let expr"),
            "do" => parse_do(first, rest).context("Parsing do expr"),
            "lambda" => parse_lambda(first, rest).context("Parsing lambda expr"),
            _ => parse_application(first, rest).context("Parsing application expr"),
        }
    } else {
        parse_application(first, rest).context("Parsing application expr")
    }?;

    Ok(r)
}

fn parse_def_single(v: &[Literal]) -> Result<Def> {
    if v.len() > 2 {
        return Err(err_msg("Excessive items after def"));
    }

    match parse_def_partial(v) {
        Ok(d) => Ok(d),
        Err(e) => Err(e),
    }
}

fn parse_def_partial(v: &[Literal]) -> Result<Def> {
    if v.len() < 2 {
        return Err(err_msg("Insufficient terms for def"));
    }

    let name;

    if let Literal::Keyword(ref s) = v[0] {
        name = s.clone();
    } else {
        return Err(err_msg("first term of def must be keyword"));
    }

    let v = parse(&v[1]).context("Second term of def must be valid AST")?;

    Ok(Def { name, value: v })
}

fn parse_if(_first: &Literal, rest: &[Literal]) -> Result<AST> {
    if rest.len() != 3 {
        return Err(err_msg("malformed if expr, (if pred then else)"));
    }

    let mut asts: Vec<Rc<AST>> = rest.iter()
        .map(|l| parse(l))
        .collect::<Result<Vec<AST>>>()? // make sure there are no parse errors
        .into_iter()
        .map(Rc::new)
        .collect();

    // These shouldn't fail, based on the length test above.
    let els = asts.pop().ok_or(err_msg("If requires else clause"))?;
    let then = asts.pop().ok_or(err_msg("If requires then clause"))?;
    let pred = asts.pop().ok_or(err_msg("If requires predicate"))?;

    Ok(AST::If { pred, then, els })
}

fn parse_def_expr(_first: &Literal, rest: &[Literal]) -> Result<AST> {
    let def = parse_def_single(rest)?;
    Ok(AST::Def(Rc::new(def)))
}

fn parse_let(_first: &Literal, rest: &[Literal]) -> Result<AST> {
    let def_literals = rest
        .get(0)
        .ok_or(err_msg(
            "let requires def list as first term (let (defs+) body)",
        ))?.ensure_list()
        .context("Parsing list of defs")?;

    let body_literal = rest.get(1).ok_or(err_msg(
        "let requires body as second term (let (defs+) body)",
    ))?;

    if rest.len() != 2 {
        return Err(err_msg("Malformed let, (let (defs+) body)"));
    }

    if def_literals.len() == 0 {
        return Err(err_msg("empty list of let bindings is not allowed"));
    }

    if def_literals.len() % 2 != 0 {
        return Err(err_msg("in let, def list must be even"));
    }

    let body = Rc::new(parse(body_literal).context("While parsing body of let")?);

    let mut defs = Vec::with_capacity(def_literals.len() / 2);

    let mut def_literals = &def_literals[..];

    // TODO: currently can't report def index
    while !def_literals.is_empty() {
        defs.push(parse_def_partial(&def_literals).context("Parsing defs in let")?);
        def_literals = &def_literals
            .get(2..)
            .ok_or(err_msg("Error slicing defs, not enough def terms"))?;
    }

    Ok(AST::Let { defs, body })
}

fn parse_do(_first: &Literal, rest: &[Literal]) -> Result<AST> {
    Ok(AST::Do(rest.iter().map(parse).collect::<Result<_>>()?))
}

fn parse_lambda(_first: &Literal, rest: &[Literal]) -> Result<AST> {
    let args = rest
        .get(0)
        .ok_or(err_msg(
            "lambda requires an argument list, (lambda (args*) body)",
        ))?.ensure_list()?
        .iter()
        .map(Literal::ensure_keyword)
        .collect::<Result<_>>()?;

    let body = rest
        .get(1)
        .ok_or(err_msg("lambda requires body, (lambda (args*) body)"))?;
    let body = Rc::new(parse(body)?);

    Ok(AST::Lambda { args, body })
}

fn parse_application(first: &Literal, rest: &[Literal]) -> Result<AST> {
    let f = Rc::new(parse(first).context("Function AST in application")?);

    let args = rest
        .iter()
        .map(parse)
        .collect::<Result<_>>()
        .context("Arguments to application")?;

    Ok(AST::Application { f, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    use data::Literal;
    use parser::Parser;
    use std::rc::Rc;

    fn p(s: &str) -> Result<Vec<Literal>> {
        let p = Parser::new();
        p.parse(s)
    }

    // parse_string
    fn ps(s: &str) -> Result<AST> {
        parse(&p(s).unwrap()[0])
    }

    #[test]
    fn test_value() {
        assert_eq!(ps("1").unwrap(), AST::Value(Literal::Number(1)));
        assert_eq!(
            parse(&Literal::Boolean(true)).unwrap(),
            AST::Value(Literal::Boolean(true))
        );
        assert!(parse(&Literal::Address((0, 0))).is_err());
    }

    #[test]
    fn test_if() {
        assert_eq!(
            ps("(if 0 0 0)").unwrap(),
            AST::If {
                pred: Rc::new(ps("0").unwrap()),
                then: Rc::new(ps("0").unwrap()),
                els: Rc::new(ps("0").unwrap()),
            }
        );

        assert!(ps("(if)").is_err());
        assert!(ps("(if 0)").is_err());
        assert!(ps("(if 0 0)").is_err());
    }

    #[test]
    fn test_def_parital() {
        let p1 = p("test 0").unwrap();

        assert_eq!(
            parse_def_partial(&p1).unwrap(),
            Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }
        );

        let p2 = p("0 0").unwrap();

        assert!(parse_def_partial(&p2).is_err());

        let p3 = p("test 0 asdf").unwrap();

        assert_eq!(
            parse_def_partial(&p3).unwrap(),
            Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }
        );
    }

    #[test]
    fn test_def_single() {
        // Mostly copied from test_def_partial
        let p1 = p("test 0").unwrap();

        assert_eq!(
            parse_def_single(&p1).unwrap(),
            Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }
        );

        let p2 = p("0 0").unwrap();

        assert!(parse_def_single(&p2).is_err());

        let p3 = p("test 0 asdf").unwrap();

        assert!(parse_def_single(&p3).is_err());
    }

    #[test]
    fn test_def() {
        let p1 = ps("(def test 0)").unwrap();

        assert_eq!(
            p1,
            AST::Def(Rc::new(Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }))
        );

        // Check errors are passed, assume other errors work
        assert!(ps("(def test)").is_err());
    }

    #[test]
    fn test_let() {
        let p1 = ps("(let (test 0) 0)").unwrap();

        assert_eq!(
            p1,
            AST::Let {
                defs: vec![Def {
                    name: "test".to_string(),
                    value: AST::Value(Literal::Number(0))
                }],
                body: Rc::new(AST::Value(Literal::Number(0)))
            }
        );

        let p2 = ps("(let (test 0 asdf 0) 0)").unwrap();

        assert_eq!(
            p2,
            AST::Let {
                defs: vec![
                    Def {
                        name: "test".to_string(),
                        value: AST::Value(Literal::Number(0))
                    },
                    Def {
                        name: "asdf".to_string(),
                        value: AST::Value(Literal::Number(0))
                    },
                ],
                body: Rc::new(AST::Value(Literal::Number(0)))
            }
        );

        let p3 = ps("(let (test 0 asdf) 0)");

        assert!(p3.is_err());

        let p4 = ps("(let (test 0))");

        assert!(p4.is_err());

        let p5 = ps("(let () 0)");

        assert!(p5.is_err());
    }

    #[test]
    fn test_var() {
        let p1 = ps("test").unwrap();

        assert_eq!(p1, AST::Var("test".to_string()));

        let p2 = ps("asdf1234").unwrap();

        assert_eq!(p2, AST::Var("asdf1234".to_string()));

        let p3 = ps("+").unwrap();

        assert_eq!(p3, AST::Var("+".to_string()));
    }

    #[test]
    fn test_do() {
        let p1 = ps("(do 0 0 0 0)").unwrap();

        assert_eq!(
            p1,
            AST::Do(vec![
                AST::Value(Literal::Number(0)),
                AST::Value(Literal::Number(0)),
                AST::Value(Literal::Number(0)),
                AST::Value(Literal::Number(0)),
            ])
        );

        let p2 = ps("(do)").unwrap();

        assert_eq!(p2, AST::Do(vec![]))
    }

    #[test]
    fn test_lambda() {
        let p1 = ps("(lambda (test) 0)").unwrap();

        assert_eq!(
            p1,
            AST::Lambda {
                args: vec!["test".to_string()],
                body: Rc::new(AST::Value(Literal::Number(0))),
            }
        );

        let p2 = ps("(lambda () 0)").unwrap();

        assert_eq!(
            p2,
            AST::Lambda {
                args: vec![],
                body: Rc::new(AST::Value(Literal::Number(0))),
            }
        );

        assert!(ps("(lambda (test))").is_err());
        assert!(ps("(lambda 0)").is_err());
    }

    #[test]
    fn test_application() {
        let p1 = ps("(+ 0 0 0)").unwrap();

        assert_eq!(
            p1,
            AST::Application {
                f: Rc::new(AST::Var("+".to_string())),
                args: vec![
                    AST::Value(Literal::Number(0)),
                    AST::Value(Literal::Number(0)),
                    AST::Value(Literal::Number(0)),
                ]
            }
        );

        let p2 = ps("(+)").unwrap();

        assert_eq!(
            p2,
            AST::Application {
                f: Rc::new(AST::Var("+".to_string())),
                args: vec![],
            }
        )
    }
}
