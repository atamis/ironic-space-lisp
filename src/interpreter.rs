
use data::Literal;
use data::list;
use errors::*;
use environment::Environment;

pub fn eval(e: &Literal) -> Result<Literal> {
    ieval(e, &Environment::new())
}

// Internal eval
fn ieval(e: &Literal, v: &Environment) -> Result<Literal> {
    match e {
        Literal::List(ref vec) => {
            if let Some((f, rest)) = vec.split_first() {
                eval_compound(f, rest, v)
            } else {
                Err("empty list not valid".into())
            }
        },
        Literal::Boolean(_) => Ok(e.clone()),
        Literal::Number(_) => Ok(e.clone()),
        Literal::Address(_) => Err("Address literals not supported".into()),
        _ => Err("Not implemented".into()),
    }
}

fn eval_compound(f: &Literal, rest: &[Literal], v: &Environment) -> Result<Literal> {
    match f {
        Literal::Keyword(s) if *s == "+".to_string() => {
            let exprs: Result<Vec<_>> = rest.iter()
                .map(|e| ieval(e, v).chain_err(|| "Evaluating arguments for +"))
                .collect();

            let exprs = exprs?;

            let ns: Result<Vec<_>> = exprs.iter()
                .map(|v| v.ensure_number())
                .collect();

            let ns = ns.chain_err(|| "All arguments to + must be numbers")?;

            Ok(Literal::Number(ns.iter().fold(0, |sum, n| sum + n)))
        }
        _ => Err("Not implemented".into(),)
    }

}


#[cfg(test)]
mod tests {
    use interpreter::eval;
    use data::Literal;
    use data::list;
    use parser::Parser;
    use errors::*;

    fn eval_string(s: &str) -> Result<Literal> {
        let p = Parser::new();
        eval(&p.parse(s).unwrap()[0])
    }

    #[test]
    fn eval_literal() {
        assert_eq!(eval(&Literal::Number(4)).unwrap(), Literal::Number(4));
        assert_eq!(eval(&Literal::Boolean(true)).unwrap(), Literal::Boolean(true));
        assert_eq!(eval(&Literal::Boolean(false)).unwrap(), Literal::Boolean(false));

        assert!(eval(&Literal::Address((0, 0))).is_err());
        assert!(eval(&list(vec![])).is_err());

        assert!(eval_string("()").is_err());
    }

    #[test]
    fn test_plus() {
        assert_eq!(eval_string("(+ 1 2 3)").unwrap(), Literal::Number(6));
        assert!(eval_string("(+ () 2 3)").is_err());
        assert!(eval_string("(())").is_err());
    }
}
