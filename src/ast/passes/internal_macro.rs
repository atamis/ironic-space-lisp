//! Convert variadic list applications to static arity cons applications
//!
//! This should be called before `unbound` because the converts `list`, which
//! has no binding, to `cons`, which is a syscall.

use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data;
use data::Keyword;
use data::Literal;
use errors::*;
use std::rc::Rc;

pub fn pass(a: &AST) -> Result<AST> {
    let mut lp = Pass {};

    lp.visit(a)
}

struct Pass;

impl Pass {
    fn visit_def(&mut self, d: &Def) -> Result<Def> {
        Ok(Def {
            name: d.name.clone(),
            value: self.visit(&d.value)?,
        })
    }

    // Vec should be reversed before being passed to consify
    // consify uses Vec::pop for better performance
    fn consify(&mut self, mut v: Vec<AST>) -> Result<AST> {
        if v.is_empty() {
            Ok(AST::Value(data::list(vec![])))
        } else {
            Ok(AST::Application {
                f: Rc::new(AST::Var("cons".to_string())),
                args: vec![v.pop().unwrap(), self.consify(v)?],
            })
        }
    }

    // Returns Ok(None) if no expansion happened
    fn expand(&mut self, s: &str, args: &[AST]) -> Result<Option<AST>> {
        match s {
            "list" => {
                let mut new_args = self.multi_visit(args)?;
                new_args.reverse();
                let new_ast = self.consify(new_args)?;
                Ok(Some(new_ast))
            }
            _ => Ok(None),
        }
    }
}

impl ASTVisitor<AST> for Pass {
    fn value_expr(&mut self, l: &Literal) -> Result<AST> {
        Ok(AST::Value(l.clone()))
    }

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<AST> {
        Ok(AST::If {
            pred: Rc::new(self.visit(pred)?),
            then: Rc::new(self.visit(then)?),
            els: Rc::new(self.visit(els)?),
        })
    }

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<AST> {
        Ok(AST::Def(Rc::new(self.visit_def(def)?)))
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<AST> {
        let new_defs = defs
            .iter()
            .map(|d| self.visit_def(d))
            .collect::<Result<_>>()?;

        Ok(AST::Let {
            defs: new_defs,
            body: Rc::new(self.visit(body)?),
        })
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<AST> {
        let new_exprs = self.multi_visit(exprs)?;

        Ok(AST::Do(new_exprs))
    }

    fn lambda_expr(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<AST> {
        Ok(AST::Lambda {
            args: args.to_vec(),
            body: Rc::new(self.visit(body)?),
        })
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<AST> {
        Ok(AST::Var(k.clone()))
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<AST> {
        if let AST::Var(ref s) = **f {
            if let Some(ast) = self.expand(s, args)? {
                return Ok(ast);
            }
        }

        let new_args = self.multi_visit(args)?;

        Ok(AST::Application {
            f: Rc::new(self.visit(f)?),
            args: new_args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast;
    use ast::AST;
    use data::list;
    use data::Literal;
    use parser;

    fn p(s: &str) -> Result<AST> {
        let p = parser::Parser::new();
        let lit = &p.parse(s)?[0];
        let ast = ast::parse(lit)?;
        pass(&ast)
    }

    #[test]
    fn test_list() {
        assert_eq!(
            p("(list 1 2)").unwrap(),
            AST::Application {
                f: Rc::new(AST::Var("cons".to_string())),
                args: vec![
                    AST::Value(Literal::Number(1)),
                    AST::Application {
                        f: Rc::new(AST::Var("cons".to_string())),
                        args: vec![AST::Value(Literal::Number(2)), AST::Value(list(vec![]))]
                    }
                ]
            }
        );

        assert_eq!(p("(list)").unwrap(), AST::Value(list(vec![])),)
    }
}
