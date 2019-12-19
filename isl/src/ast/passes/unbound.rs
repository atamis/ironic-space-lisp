//! Determine if any variables are unbound in an [`AST`](super::AST).
use crate::ast::ASTVisitor;
use crate::ast::Def;
use crate::ast::AST;
use crate::data::Literal;
use crate::data::Symbol;
use crate::env::Env;
use crate::errors::*;
use im::hashset;
use std::rc::Rc;

const OP_FUNCS: &[&str] = &["fork", "wait", "send", "pid", "terminate"];

#[allow(dead_code)]
type SymbolSet = hashset::HashSet<Symbol>;

/// Do the pass. See [`super::unbound`] for more information.
///
/// This checks unbound variables with an empty environment. This also checks a slice of [`AST`]s together.
pub fn pass_default(asts: &[AST]) -> Result<()> {
    let mut hs = hashset::HashSet::new();

    asts.iter().map(|a| hs.visit(a)).collect()
}

/// Do the pass. See [`super::unbound`] for more information.
///
/// Check variables against an existing environment.
pub fn pass(ast: &AST, env: &Env) -> Result<()> {
    let mut hs: SymbolSet = env.keys().cloned().collect();

    for op_key in OP_FUNCS.iter().map(|s| *s) {
        hs.insert(op_key.to_string());
    }

    hs.visit(ast).context("Pass with specific env")?;
    Ok(())
}

impl ASTVisitor<()> for SymbolSet {
    fn value_expr(&mut self, _l: &Literal) -> Result<()> {
        Ok(())
    }

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<()> {
        self.visit(pred).context("Visiting predicate")?;
        self.visit(then).context("Vising then arm")?;
        self.visit(els).context("Vising else arm")?;
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
            c.visit(&d.value)?;
        }

        c.visit(body)
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<()> {
        for a in exprs {
            if let AST::Def(d) = a {
                self.insert(d.name.clone());
            }
        }

        self.multi_visit(exprs).context("Do expressions")?;
        Ok(())
    }

    fn lambda_expr(&mut self, args: &[Symbol], body: &Rc<AST>) -> Result<()> {
        let mut c = self.clone();
        for k in args {
            c.insert(k.clone());
        }

        c.visit(body).context("Visiting lambda body")?;
        Ok(())
    }

    fn var_expr(&mut self, k: &Symbol) -> Result<()> {
        if self.contains(k) {
            Ok(())
        } else {
            Err(format_err!("Unbound var {:}", k))
        }
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<()> {
        self.visit(f).context("Function applicable expr")?;
        self.multi_visit(args).context("Arguments to application")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::pass_default;
    use crate::ast;
    use crate::ast::AST;
    use crate::errors::*;
    use crate::parser;

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
        // Recursive definition
        // TODO: maybe try to differentiate between post-binding access?
        //assert!(p("(def test test)").is_err());

        assert!(p("(def test 0) test").is_ok())
    }

    #[test]
    fn test_let() {
        assert!(p("(let (test 0) asdf)").is_err());
        assert!(p("(let (test 0) test)").is_ok());
        assert!(p("(let (test 1) (let (asdf test) asdf))").is_ok());
        assert!(p("(let (test 1) (let (asdf nottest) asdf))").is_err());
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
