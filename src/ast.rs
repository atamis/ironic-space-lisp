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

pub fn parse(e: &Literal) -> Result<AST> {
    match e {
        Literal::List(ref vec) => {
            if let Some((first, rest)) = vec.split_first() {
                parse_compound(first, rest)
            } else {
                Err("empty list not valid".into())
            }
        }
        Literal::Keyword(k) => Ok(AST::Var(k.clone())),
        Literal::Boolean(_) => Ok(AST::Value(e.clone())),
        Literal::Number(_) => Ok(AST::Value(e.clone())),
        Literal::Address(_) => Err("Address literals not supported".into()),
    }
}

fn parse_compound(first: &Literal, rest: &[Literal]) -> Result<AST> {
    if let Literal::Keyword(s) = first {
        match s.as_ref() {
            "if" => {
                if rest.len() != 3 {
                    return Err("malformed if expr, (if pred then else)".into());
                }

                let mut asts: Vec<Rc<AST>> = rest.iter()
                    .map(|l| parse(l))
                    .collect::<Result<Vec<AST>>>()? // make sure there are no parse errors
                    .into_iter()
                    .map(Rc::new)
                    .collect();

                // These shouldn't fail, based on the length test above.
                let els = asts.pop().ok_or("If requires else clause")?;
                let then = asts.pop().ok_or("If requires then clause")?;
                let pred = asts.pop().ok_or("If requires predicate")?;

                Ok(AST::If { pred, then, els })
            }
            "def" => {
                let def = parse_def_single(rest)?;
                Ok(AST::Def(Rc::new(def)))
            }
            "let" => {
                let mut def_literals = rest
                    .get(0)
                    .ok_or("let requires def list as first term (let (defs+) body)")?
                    .ensure_list()?;

                let body_literal = rest
                    .get(1)
                    .ok_or("let requires body as second term (let (defs+) body)")?;

                if rest.len() != 2 {
                    return Err("Malformed let, (let (defs+) body)".into());
                }

                if def_literals.len() == 0 {
                    return Err("empty list of let bindings is not allowed".into());
                }

                if def_literals.len() % 2 != 0 {
                    return Err("in let, def list must be even".into());
                }

                let body = Rc::new(parse(body_literal)?);

                let mut defs = Vec::with_capacity(def_literals.len() / 2);

                let mut def_literals = &def_literals[..];

                while !def_literals.is_empty() {
                    defs.push(parse_def_partial(&def_literals)?);
                    def_literals = &def_literals
                        .get(2..)
                        .ok_or("Error slicing defs, not enough def terms")?;
                }

                Ok(AST::Let { defs, body })
            }
            "do" => Ok(AST::Do(rest.iter().map(parse).collect::<Result<_>>()?)),
            "lambda" => {
                let args = rest
                    .get(0)
                    .ok_or("lambda requires an argument list, (lambda (args*) body)")?
                    .ensure_list()?
                    .iter()
                    .map(Literal::ensure_keyword)
                    .collect::<Result<_>>()?;

                let body = rest
                    .get(1)
                    .ok_or("lambda requires body, (lambda (args*) body)")?;
                let body = Rc::new(parse(body)?);

                Ok(AST::Lambda { args, body })
            }
            _ => {
                let f = Rc::new(parse(first).chain_err(|| "Function AST in application")?);

                let args = rest
                    .iter()
                    .map(parse)
                    .collect::<Result<_>>()
                    .chain_err(|| "Arguments to application")?;

                Ok(AST::Application { f, args })
            }
        }
    } else {
        Err("Not implemented".into())
    }
}

fn parse_def_single(v: &[Literal]) -> Result<Def> {
    if v.len() > 2 {
        return Err("Excessive items after def".into());
    }

    match parse_def_partial(v) {
        Ok(d) => Ok(d),
        Err(e) => Err(e),
    }
}

fn parse_def_partial(v: &[Literal]) -> Result<Def> {
    if v.len() < 2 {
        return Err("Insufficient terms for def".into());
    }

    let name;

    if let Literal::Keyword(ref s) = v[0] {
        name = s.clone();
    } else {
        return Err("first term of def must be keyword".into());
    }

    let v = parse(&v[1]).chain_err(|| "Second term of def must be valid AST")?;

    Ok(Def { name, value: v })
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
