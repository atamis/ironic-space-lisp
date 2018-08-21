use std::rc::Rc;

use ast::Def;
use ast::AST;
use data::Literal;
use environment::Env;
use errors::*;

#[derive(Default)]
pub struct Interpreter {
    pub global: Env,
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            global: Interpreter::default_environment(),
        }
    }

    pub fn with_env(env: Env) -> Interpreter {
        Interpreter { global: env }
    }

    fn default_environment() -> Env {
        let mut e = Env::new();
        e.insert("true".to_string(), Rc::new(Literal::Boolean(true)));
        e.insert("false".to_string(), Rc::new(Literal::Boolean(false)));
        e
    }

    pub fn eval(&mut self, a: &AST) -> Result<Literal> {
        let mut ng = self.global.clone();
        let res = self.env_eval(a, &mut ng)?;
        self.global = ng;
        Ok(res)
    }

    pub fn env_eval(&self, a: &AST, env: &mut Env) -> Result<Literal> {
        match a {
            AST::Value(l) => Ok(l.clone()),
            AST::Var(k) => {
                let r = env
                    .get(k)
                    .chain_err(|| format!("While accessing var {:}", k))?;

                // This code ends up cloning twice, and I don't know how to do it better.
                Ok((**r).clone())
            }
            AST::If { pred, then, els } => {
                let pv = self
                    .env_eval(pred, env)
                    .chain_err(|| "Evaluating predicate for if")?;

                if pv.truthy() {
                    Ok(self
                        .env_eval(then, env)
                        .chain_err(|| "Evaluating then for if")?)
                } else {
                    Ok(self
                        .env_eval(els, env)
                        .chain_err(|| "Evaluating else for if")?)
                }
            }
            AST::Def(ref def) => {
                let res = self.put_def(env, def).chain_err(|| "Evaluating def ")?;
                Ok(res)
            }
            AST::Let { defs, body } => {
                let mut let_env = env.clone();

                for d in defs {
                    self.put_def(&mut let_env, d)
                        .chain_err(|| "Evalutaing bindings for let")?;
                }

                let body_val = self
                    .env_eval(body, &mut let_env)
                    .chain_err(|| "Evaluting let body")?;

                Ok(body_val)
            }
            AST::Do(asts) => {
                let mut vals: Vec<Literal> = asts
                    .iter()
                    .map(|e| self.env_eval(e, env))
                    .collect::<Result<_>>()
                    .chain_err(|| "Evaluating do sub-expressions")?;
                Ok(vals.pop().chain_err(|| "do expressions can't be empty")?)
            }
            _ => Err("Not implemented".into()),
        }
    }

    fn put_def(&self, env: &mut Env, def: &Def) -> Result<Literal> {
        let res = self
            .env_eval(&def.value, env)
            .chain_err(|| format!("While evaluating def value for {:}", def.name.clone()))?;
        env.insert(def.name.clone(), Rc::new(res.clone()));
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast;
    use data::Literal;
    use errors::*;
    use parser::Parser;

    fn pi(i: &mut Interpreter, s: &str) -> Result<Literal> {
        let p = Parser::new();
        let lits = &p.parse(s).unwrap()[0];
        let ast = &ast::parse(lits).unwrap();
        i.eval(ast)
    }

    #[test]
    fn eval_literal() {
        let mut i = Interpreter::new();
        assert_eq!(pi(&mut i, "4").unwrap(), Literal::Number(4));
    }

    #[test]
    fn eval_boolean() {
        let mut i = Interpreter::new();
        assert_eq!(pi(&mut i, "true").unwrap(), Literal::Boolean(true));
        assert_eq!(pi(&mut i, "false").unwrap(), Literal::Boolean(false));
    }

    #[test]
    fn test_if() {
        let mut i = Interpreter::new();

        let p1 = pi(&mut i, "(if true 1 0)").unwrap();

        assert_eq!(p1, Literal::Number(1));

        let p2 = pi(&mut i, "(if false 1 0)").unwrap();

        assert_eq!(p2, Literal::Number(0));
    }

    #[test]
    fn test_def() {
        let mut i = Interpreter::new();

        let p1 = pi(&mut i, "(def test 5)").unwrap();

        assert_eq!(p1, Literal::Number(5));
        assert_eq!(pi(&mut i, "test").unwrap(), Literal::Number(5));
    }

    #[test]
    fn test_let() {
        let mut i = Interpreter::new();
        let p1 = pi(&mut i, "(let (test 5) test)").unwrap();
        assert_eq!(p1, Literal::Number(5));
        assert!(pi(&mut i, "test").is_err());
        assert!(i.global.get(&"test".to_string()).is_none());
    }

    #[test]
    fn test_do() {
        let mut i = Interpreter::new();

        let p1 = pi(&mut i, "(do 1 2 3)").unwrap();
        assert_eq!(p1, Literal::Number(3));

        let p2 = pi(&mut i, "(do (def test 4) test)").unwrap();
        assert_eq!(p2, Literal::Number(4));
    }

}
