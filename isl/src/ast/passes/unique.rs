//! [`LiftedAST`](function_lifter::LiftedAST) pass to rename local rebindings to unique names

use crate::ast::passes::function_lifter;
use crate::ast::passes::function_lifter::ASTFunction;
use crate::ast::passes::function_lifter::LASTVisitor;
use crate::ast::passes::function_lifter::LiftedAST;
use crate::ast::ASTVisitor;
use crate::ast::Def;
use crate::ast::DefVisitor;
use crate::ast::AST;
use crate::data::Literal;
use crate::data::Symbol;
use crate::errors::*;
use im::hashmap::HashMap;
use im::hashset::HashSet;
use std::rc::Rc;

/// Do the pass. See [`super::unique`] for more information.
pub fn pass(last: &LiftedAST) -> Result<LiftedAST> {
    let mut l: Unique = Default::default();

    let fns = l.last_visit(last)?;

    Ok(LiftedAST {
        fr: function_lifter::FunctionRegistry { functions: fns },
        entry: last.entry,
    })
}

#[derive(Default, Clone)]
struct Unique {
    bindings: HashSet<Symbol>,
    renames: HashMap<Symbol, Symbol>,
    top_level_defs: bool,
}

impl Unique {
    fn convert_fn(&mut self, args: &[Symbol], body: &Rc<AST>) -> Result<ASTFunction> {
        Ok(ASTFunction {
            args: args.to_vec(),
            body: Rc::new(self.visit(body)?),
        })
    }
}

impl LASTVisitor<ASTFunction> for Unique {
    fn ast_function(&mut self, args: &[Symbol], body: &Rc<AST>) -> Result<ASTFunction> {
        let mut u = self.clone();

        for k in args {
            u.bindings.insert(k.to_string());
        }

        u.convert_fn(args, body)
    }

    fn ast_function_entry(&mut self, args: &[Symbol], body: &Rc<AST>) -> Result<ASTFunction> {
        let mut u = self.clone();

        u.top_level_defs = true;

        u.convert_fn(args, body)
    }
}

impl DefVisitor<Def> for Unique {
    fn visit_def(&mut self, name: &str, value: &AST) -> Result<Def> {
        if self.bindings.contains(name) {
            use rand::prelude::*;
            let i: usize = thread_rng().gen();

            let new_name = format!("{}_{}", name, i);

            self.bindings.insert(new_name.to_string());
            self.renames.insert(name.to_string(), new_name);

            Ok(Def {
                name: format!("{}_{}", name, i),
                value: self.visit(value)?,
            })
        } else {
            self.bindings.insert(name.to_string());

            Ok(Def {
                name: name.to_string(),
                value: self.visit(value)?,
            })
        }
    }
}

impl ASTVisitor<AST> for Unique {
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
        if self.top_level_defs {
            let mut u = self.clone();
            u.top_level_defs = true;

            let value = self.visit(&def.value)?;
            Ok(AST::Def(Rc::new(Def {
                name: def.name.to_string(),
                value,
            })))
        } else {
            Ok(AST::Def(Rc::new(self.visit_single_def(def)?)))
        }
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<AST> {
        let mut subenv = self.clone();
        subenv.top_level_defs = false;
        let newdefs = subenv.visit_multi_def(defs)?;
        let newbody = subenv.visit(body)?;

        Ok(AST::Let {
            defs: newdefs,
            body: Rc::new(newbody),
        })
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<AST> {
        let exprs = self.multi_visit(exprs)?;
        Ok(AST::Do(exprs))
    }

    fn lambda_expr(&mut self, _args: &[Symbol], _body: &Rc<AST>) -> Result<AST> {
        Err(err_msg("lambda exprs not supported"))
    }

    fn var_expr(&mut self, k: &Symbol) -> Result<AST> {
        Ok(AST::Var(match self.renames.get(k) {
            Some(new_name) => new_name.clone(),
            None => k.to_string(),
        }))
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<AST> {
        let f = self.visit(f)?;
        let args = self.multi_visit(args)?;
        Ok(AST::Application {
            f: Rc::new(f),
            args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::ast::passes::function_lifter;
    use crate::ast::passes::internal_macro;
    use crate::parser;

    fn do_pass(s: &str) -> Result<LiftedAST> {
        let ast = ast::parse_multi(&parser::parse(s).unwrap()).unwrap();

        let ast = internal_macro::pass(&ast).unwrap();

        let last = function_lifter::lift_functions(&ast).unwrap();

        pass(&last)
    }

    // TODO: destructuring these enums is obnoxiously hard, so I'm going to trust
    // that the visitors recur properly.

    #[test]
    fn test_let_rebinding() {
        let last1 = do_pass("(let (x 2) (let (x 1) x))").unwrap();
        let f1 = &last1.fr.functions[0];

        if let AST::Let { defs: _, ref body } = *f1.body {
            if let AST::Let { ref defs, body: _ } = **body {
                assert_ne!("x", defs[0].name);
            } else {
                panic!();
            }
        } else {
            panic!();
        }

        println!("{:?}", do_pass("(def x 1) (def x 2)"));
        //assert!(false);
    }

    #[test]
    fn test_internal_defs() {
        let last1 = do_pass("(let (x 2) (do (def x 1) x))").unwrap();
        let f1 = &last1.fr.functions[0];

        if let AST::Let { defs: _, ref body } = *f1.body {
            if let AST::Do(ref exprs) = **body {
                let def = &exprs[0];
                let varref = &exprs[1];

                // Get def name
                let name1 = if let AST::Def(ref d) = def {
                    &d.name
                } else {
                    panic!()
                };

                // Get name varref name
                let name2 = if let AST::Var(k) = varref {
                    k
                } else {
                    panic!()
                };

                // The same because they refer to the same local var
                assert_eq!(name1, name2);
                // Different because it's a rebinding.
                assert_ne!("x", name1);
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }
}
