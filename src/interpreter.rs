use std::rc::Rc;

use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data::Keyword;
use data::Literal;
use environment::Env;
use errors::*;

#[derive(Default)]
pub struct Interpreter {
    pub global: Env,
}

impl ASTVisitor<Literal> for Env {
    fn value_expr(&mut self, l: &Literal) -> Result<Literal> {
        Ok(l.clone())
    }

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<Literal> {
        let pv = self.visit(pred).context("Evaluating predicate for if")?;

        if pv.truthy() {
            Ok(self.visit(then).context("Evaluating then for if")?)
        } else {
            Ok(self.visit(els).context("Evaluating else for if")?)
        }
    }

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<Literal> {
        let res = put_def(self, def).context("Evaluating def")?;

        Ok(res)
    }

    fn let_expr(&mut self, defs: &Vec<Def>, body: &Rc<AST>) -> Result<Literal> {
        let mut let_env = self.clone();

        for d in defs {
            // TODO binding index
            put_def(&mut let_env, d).context("Evalutaing bindings for let")?;
        }

        let body_val = let_env.visit(body).context("Evaluting let body")?;

        Ok(body_val)
    }

    fn do_expr(&mut self, exprs: &Vec<AST>) -> Result<Literal> {
        let mut vals: Vec<Literal> = exprs
            .iter()
            .map(|e| self.visit(e))
            .collect::<Result<_>>()
            .context("Evaluating do sub-expressions")?;
        Ok(vals.pop().ok_or(err_msg("do expressions can't be empty"))?)
    }

    fn lambda_expr(&mut self, _args: &Vec<Keyword>, _body: &Rc<AST>) -> Result<Literal> {
        Err(err_msg("Not implemented"))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<Literal> {
        let r = self.get(k).ok_or(format_err!("While access var {:}", k))?;

        Ok((**r).clone())
    }

    fn application_expr(&mut self, _f: &Rc<AST>, _args: &Vec<AST>) -> Result<Literal> {
        Err(err_msg("Not implemented"))
    }
}

fn put_def(env: &mut Env, def: &Def) -> Result<Literal> {
    let res = env.visit(&def.value).context(format_err!(
        "While evaluating def value for {:}",
        def.name.clone()
    ))?;
    env.insert(def.name.clone(), Rc::new(res.clone()));
    Ok(res)
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
        env.visit(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast;
    use data::Literal;
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
