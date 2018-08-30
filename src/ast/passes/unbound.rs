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

pub fn pass(ast: &AST, env: &Env) -> Result<()> {
    let mut hs: KeywordSet = env.keys().cloned().collect();

    hs.visit(ast).context("Pass with specific env")?;
    Ok(())
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
        self.insert(def.name.clone());
        self.visit(&def.value)?;
        Ok(())
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<()> {
        let mut c = self.clone();
        for d in defs {
            c.insert(d.name.clone());
        }

        c.visit(body)
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<()> {
        self.multi_visit(exprs)?;
        Ok(())
    }

    fn lambda_expr(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<()> {
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

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<()> {
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
