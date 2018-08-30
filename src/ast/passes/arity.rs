use ast::passes::function_lifter::LASTVisitor;
use ast::passes::function_lifter::LiftedAST;
use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data::Keyword;
use data::Literal;
use errors::*;
use std::rc::Rc;


/*
This code is pretty broken. I don't think a compile-time arity checker is possible.
Would need access to runtime environment bindings, syscall arities. Would need to
keep track of rebindings.

Most critically, it needs access to runtime function arities, but by runtime, such
arity information has been destroyed by the compiler.

*/


pub fn pass(last: &LiftedAST) -> Result<()> {
    let mut ap = ArityPass::new(last);
    ap.last_visit(&last).context("In the arity pass")?;

    Ok(())
}

struct ArityPass<'a> {
    last: &'a LiftedAST,
}

impl<'a> ArityPass<'a> {
    pub fn new(last: &'a LiftedAST) -> ArityPass<'a> {
        ArityPass { last }
    }

    fn visit_def(&mut self, def: &Def) -> Result<()> {
        self.visit(&def.value)
    }
}

impl<'a> LASTVisitor<()> for ArityPass<'a> {
    fn ast_function(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<()> {
        self.visit(body)
    }

    fn ast_function_entry(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<()> {
        self.ast_function(args, body)
    }
}

impl<'a> ASTVisitor<()> for ArityPass<'a> {
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
        self.visit_def(def).context("Visiting def")?;
        Ok(())
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<()> {
        defs.iter()
            .map(|d| self.visit_def(d))
            .collect::<Result<_>>()
            .context("Visiting defs in let")?;
        self.visit(body).context("Visiting body of let")?;
        Ok(())
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<()> {
        self.multi_visit(exprs).context("Visiting do expr")?;
        Ok(())
    }

    fn lambda_expr(&mut self, _args: &[Keyword], _body: &Rc<AST>) -> Result<()> {
        Err(err_msg("Malformed LiftedAST, encountered lambda"))
    }

    fn var_expr(&mut self, _k: &Keyword) -> Result<()> {
        Ok(())
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<()> {
        println!("{:?}", f);
        if let AST::Value(Literal::Address(addr)) = **f {
            let func = self
                .last
                .fr
                .lookup(addr)
                .ok_or_else(|| format_err!("Function not found {:?}", addr))?;
            if func.arity() != args.len() {
                Err(err_msg("Arity missmatch"))
            } else {
                Ok(())
            }
        } else {
            Err(err_msg("Indeterminant arity"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use str_to_ast;

    fn p(s: &str) -> bool {
        use ast::passes::function_lifter;

        let ast = str_to_ast(s).unwrap();
        let last = function_lifter::lift_functions(&ast).unwrap();
        pass(&last).is_ok()
    }

    #[test]
    fn test_arity_pass() {
        assert!(p("(let (x 0) x)"));
        assert!(p("(+ 1 1)"));
        assert!(!p("(+ 1 1 1)")); // not variadic
        assert!(p("(cons 1 '(1))"));
        assert!(!p("(cons 1 1 '(1))"));
    }
}
