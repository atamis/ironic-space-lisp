//! Pass to lift functions out of the `AST` and into a function registry.
use std::rc::Rc;

use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data::Address;
use data::Keyword;
use data::Literal;
use errors::*;

/// Represents a function as a list of arguments and an `AST` node.
#[derive(Clone, Debug, PartialEq)]
pub struct ASTFunction {
    pub args: Vec<Keyword>,
    pub body: Rc<AST>,
}

impl ASTFunction {
    pub fn arity(&self) -> usize {
        self.args.len()
    }
}

/// Extracts functions from `a` to form a `LiftedAST`.
///
/// Note that this manipulates or otherwise copies all the nodes
/// in the AST, and can result in significant memory allocation.
pub fn lift_functions(a: &AST) -> Result<LiftedAST> {
    let mut fr = FunctionRegistry::new();
    let root = fr.visit(a)?;

    fr.functions[0].body = Rc::new(root);

    Ok(LiftedAST { fr, entry: 0 })
}

/// An AST with its functions lifted out.
///
/// Includes a `root` AST, and a registry containing all the functions
/// lifted out. The first function is a dummy function.
#[derive(Clone, Debug)]
pub struct LiftedAST {
    pub fr: FunctionRegistry,
    pub entry: usize,
}

impl LiftedAST {
    pub fn entry_fn(&self) -> &ASTFunction {
        &self.fr.functions[self.entry]
    }

    pub fn import(&mut self, last: &LiftedAST) -> Result<Address> {
        let new_idx = self.fr.functions.len();
        let import_entry = last.entry;
        let new_entry = import_entry + new_idx;

        let mut new_fns = import::Import(new_idx)
            .last_visit(last)
            .context("While importing functions from a LiftedAST")?;

        self.fr.functions.append(&mut new_fns);

        Ok((new_entry, 0))
    }
}

/// Represents a registry of functions for some `AST`.
///
/// Stored as a vector of `ASTFunctions` where the index of the function
/// in the vector is assumed to be its future address in the form `(idx, 0)`.
/// This is a naive method of function registry to go with the naive code
/// packer in `compiler::pack_compile_lifted`.
#[derive(Clone, Debug, Default)]
pub struct FunctionRegistry {
    pub functions: Vec<ASTFunction>,
}

impl FunctionRegistry {
    pub fn new() -> FunctionRegistry {
        FunctionRegistry {
            functions: vec![ASTFunction {
                args: vec![],
                body: Rc::new(AST::Value(Literal::Boolean(false))),
            }],
        }
    }

    /// Insert a function into the registry and return its index.
    pub fn add_function(&mut self, f: ASTFunction) -> usize {
        let idx = self.functions.len();
        self.functions.push(f);
        idx
    }

    pub fn lookup(&self, addr: Address) -> Option<&ASTFunction> {
        self.functions.get(addr.0)
    }

    fn visit_def(&mut self, d: &Def) -> Result<Def> {
        Ok(Def {
            name: d.name.clone(),
            value: self.visit(&d.value)?,
        })
    }
}

impl ASTVisitor<AST> for FunctionRegistry {
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
        let new_body = Rc::new(self.visit(body)?);
        let i = self.add_function(ASTFunction {
            args: args.to_vec(),
            body: new_body,
        });

        Ok(AST::Value(Literal::Closure(args.len(), (i, 0))))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<AST> {
        Ok(AST::Var(k.clone()))
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<AST> {
        Ok(AST::Application {
            f: Rc::new(self.visit(f)?),
            args: args.iter().map(|e| self.visit(e)).collect::<Result<_>>()?,
        })
    }
}

pub trait LASTVisitor<T> {
    fn last_visit(&mut self, last: &LiftedAST) -> Result<Vec<T>> {
        let entry = last.entry;
        let rs = last
            .fr
            .functions
            .iter()
            .enumerate()
            .map(|(idx, func)| {
                let res = if idx == entry {
                    self.ast_function_entry(&func.args, &func.body)
                        .context(format!("While visiting function {:}", idx))
                } else {
                    self.ast_function(&func.args, &func.body)
                        .context(format!("While visiting function {:}", idx))
                }?;

                Ok(res)
            }).collect::<Result<_>>()
            .context("While visiting LiftedAST")?;

        Ok(rs)
    }

    fn ast_function(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<T>;
    fn ast_function_entry(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<T> {
        self.ast_function(args, body)
    }
}

mod import {
    use super::*;

    pub struct Import(pub usize);

    impl Import {
        fn visit_def(&mut self, d: &Def) -> Result<Def> {
            Ok(Def {
                name: d.name.clone(),
                value: self.visit(&d.value)?,
            })
        }
    }

    impl LASTVisitor<ASTFunction> for Import {
        fn ast_function(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<ASTFunction> {
            Ok(ASTFunction {
                args: args.to_vec(),
                body: Rc::new(self.visit(body).context("Visiting body of function")?),
            })
        }
    }

    impl ASTVisitor<AST> for Import {
        fn value_expr(&mut self, l: &Literal) -> Result<AST> {
            Ok(AST::Value(match l {
                Literal::Address((a1, a2)) => (a1 + self.0, *a2).into(),
                Literal::Closure(arity, (a1, a2)) => Literal::Closure(*arity, (a1 + self.0, *a2)),
                x => x.clone(),
            }))
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

        fn lambda_expr(&mut self, _args: &[Keyword], _body: &Rc<AST>) -> Result<AST> {
            Err(err_msg("Not implemented"))
        }

        #[allow(clippy::ptr_arg)]
        fn var_expr(&mut self, k: &Keyword) -> Result<AST> {
            Ok(AST::Var(k.clone()))
        }

        fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<AST> {
            let new_args = self.multi_visit(args)?;

            Ok(AST::Application {
                f: Rc::new(self.visit(f)?),
                args: new_args,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast;
    use ast::passes::unbound::pass_default;
    use ast::AST;
    use parser;

    fn p(s: &str) -> Result<LiftedAST> {
        let p = parser::Parser::new();
        let lit = &p.parse(s)?;
        let asts: Vec<AST> = lit.iter().map(ast::parse).collect::<Result<_>>()?;
        pass_default(asts.as_ref())?;
        let ast = AST::Do(asts);
        lift_functions(&ast)
    }

    #[test]
    fn test_normal() {
        p("(let (x 1 y 2) x)").unwrap();
    }

    #[test]
    fn test_lambda() {
        let last = p("(lambda (x) x)").unwrap();

        assert_eq!(
            last.fr.functions[1],
            ASTFunction {
                args: vec!["x".to_string()],
                body: Rc::new(AST::Var("x".to_string()))
            }
        );

        assert_eq!(
            *last.entry_fn().body,
            AST::Do(vec![AST::Value(Literal::Closure(1, (1, 0)))])
        );
    }

    #[test]
    fn test_nested_lambda() {
        let last = p("(lambda (x) (lambda (y) y))").unwrap();

        assert_eq!(
            last.fr.functions[2],
            ASTFunction {
                args: vec!["x".to_string()],
                body: Rc::new(AST::Value(Literal::Closure(1, (1, 0))))
            }
        );

        assert_eq!(
            last.fr.functions[1],
            ASTFunction {
                args: vec!["y".to_string()],
                body: Rc::new(AST::Var("y".to_string()))
            }
        );

        assert_eq!(
            *last.entry_fn().body,
            AST::Do(vec![AST::Value(Literal::Closure(1, (2, 0)))])
        );
    }

    #[test]
    fn test_last_import() {
        let mut last1 = LiftedAST {
            fr: FunctionRegistry {
                functions: vec![ASTFunction {
                    args: vec![],
                    body: Rc::new(AST::Value((0, 0).into())),
                }],
            },
            entry: 0,
        };

        let last2 = LiftedAST {
            fr: FunctionRegistry {
                functions: vec![ASTFunction {
                    args: vec!["test".to_string()],
                    body: Rc::new(AST::Value((0, 0).into())),
                }],
            },
            entry: 0,
        };

        let new_entry = last1.import(&last2).unwrap();

        assert_eq!(new_entry, (1, 0));

        let new_entry_fn = &last1.fr.functions[1];

        assert_eq!(new_entry_fn.args, vec!["test"]);

        assert_eq!(*new_entry_fn.body, AST::Value((1, 0).into()));

        let orig_entry_fn = last1.entry_fn();

        assert_eq!(*orig_entry_fn.body, AST::Value((0, 0).into()));
    }
}
