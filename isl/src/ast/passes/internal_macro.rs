//! Apply several internal macros to the AST.
//!
//! Converts variadic list applications to static arity cons applications.
//! Also converts cond to nested ifs.
//!
//! This should be called before `unbound` because it converts `list`, which
//! has no binding, to `cons`, which is a syscall.

use crate::ast::ASTVisitor;
use crate::ast::Def;
use crate::ast::DefVisitor;
use crate::ast::AST;
use crate::data;
use crate::data::Keyword;
use crate::data::Literal;
use crate::errors::*;
use crate::util::*;
use std::rc::Rc;

/// Do the pass over a normal [`AST`]. See [`internal_macro`](super::internal_macro) for more information.
pub fn pass(a: &AST) -> Result<AST> {
    let mut lp = Pass {};

    lp.visit(a)
}

struct Pass;

impl Pass {
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

    fn condify(&mut self, mut terms: Vec<(AST, AST)>) -> Result<AST> {
        if terms.is_empty() {
            Ok(AST::Value(Literal::Keyword(
                "incomplete-cond-use-true".to_string(),
            )))
        } else {
            let (pred, then) = terms
                .pop()
                .ok_or_else(|| err_msg("Attempted to pop empty term list, empty check failed"))?;
            let (pred, then) = (Rc::new(pred), Rc::new(then));
            Ok(AST::If {
                pred,
                then,
                els: Rc::new(self.condify(terms)?),
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
            "cond" => {
                if args.len() % 2 != 0 {
                    return Err(err_msg(
                        "Odd number of terms in cond, even number required, (cond pred then...)",
                    ));
                }

                let mut terms: Vec<(AST, AST)> = self
                    .multi_visit(args)?
                    .into_iter()
                    .group_by_2(true)
                    .collect();
                terms.reverse();

                Ok(Some(self.condify(terms)?))
            }
            _ => Ok(None),
        }
    }
}

// WARN: panics if v.len() % 2 != 0
//fn group_by_2<T>(mut v: Vec<T>) -> Vec<(T, T)> {
//assert!(v.len() % 2 == 0);
//let mut out = Vec::with_capacity(v.len() / 2);
//
//v.reverse();
//
//while !v.is_empty() {
//let t = (v.pop().unwrap(), v.pop().unwrap());
//
//out.push(t);
//}
//
//out
//}

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
        Ok(AST::Def(Rc::new(self.visit_single_def(def)?)))
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<AST> {
        let new_defs = self.visit_multi_def(defs)?;

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

impl DefVisitor<Def> for Pass {
    fn visit_def(&mut self, name: &str, value: &AST) -> Result<Def> {
        Ok(Def {
            name: name.to_string(),
            value: self.visit(value)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::ast::AST;
    use crate::data::list;
    use crate::data::Literal;
    use crate::parser;

    fn p(s: &str) -> Result<AST> {
        let p = parser::Parser::new();
        let lit = &p.parse(s)?[0];
        let ast = ast::parse(lit)?;
        pass(&ast)
    }

    fn n(n: u32) -> AST {
        AST::Value(Literal::Number(n))
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

    #[test]
    fn test_cond() {
        assert_eq!(
            p("(cond 1 2 3 4)").unwrap(),
            AST::If {
                pred: Rc::new(n(1)),
                then: Rc::new(n(2)),
                els: Rc::new(AST::If {
                    pred: Rc::new(n(3)),
                    then: Rc::new(n(4)),
                    els: Rc::new(AST::Value(Literal::Keyword(
                        "incomplete-cond-use-true".to_string()
                    )))
                })
            }
        );
    }
}
