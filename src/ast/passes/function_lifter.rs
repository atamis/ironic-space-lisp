use std::rc::Rc;

use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data::Keyword;
use data::Literal;
use errors::*;

#[derive(Debug, PartialEq)]
pub struct ASTFunction {
    args: Vec<Keyword>,
    body: Rc<AST>,
}

pub fn lift_functions(a: &AST) -> Result<LiftedAST> {
    let mut fr = FunctionRegistry::new();
    let root = fr.visit(a)?;

    Ok(LiftedAST { fr, root })
}

pub struct FunctionRegistry {
    functions: Vec<ASTFunction>,
}

pub struct LiftedAST {
    root: AST,
    fr: FunctionRegistry,
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

    pub fn add_function(&mut self, f: ASTFunction) -> usize {
        let idx = self.functions.len();
        self.functions.push(f);
        idx
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

    fn let_expr(&mut self, defs: &Vec<Def>, body: &Rc<AST>) -> Result<AST> {
        let new_defs = defs
            .iter()
            .map(|d| self.visit_def(d))
            .collect::<Result<_>>()?;

        Ok(AST::Let {
            defs: new_defs,
            body: Rc::new(self.visit(body)?),
        })
    }

    fn do_expr(&mut self, exprs: &Vec<AST>) -> Result<AST> {
        let new_exprs = exprs.iter().map(|e| self.visit(e)).collect::<Result<_>>()?;

        Ok(AST::Do(new_exprs))
    }

    fn lambda_expr(&mut self, args: &Vec<Keyword>, body: &Rc<AST>) -> Result<AST> {
        let new_body = Rc::new(self.visit(body)?);
        let i = self.add_function(ASTFunction {
            args: args.clone(),
            body: new_body,
        });

        Ok(AST::Value(Literal::Address((i, 0))))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<AST> {
        Ok(AST::Var(k.clone()))
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &Vec<AST>) -> Result<AST> {
        Ok(AST::Application {
            f: Rc::new(self.visit(f)?),
            args: args.iter().map(|e| self.visit(e)).collect::<Result<_>>()?,
        })
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
            last.root,
            AST::Do(vec![AST::Value(Literal::Address((1, 0)))])
        );
    }

    #[test]
    fn test_nested_lambda() {
        let last = p("(lambda (x) (lambda (y) y))").unwrap();

        assert_eq!(
            last.fr.functions[2],
            ASTFunction {
                args: vec!["x".to_string()],
                body: Rc::new(AST::Value(Literal::Address((1, 0))))
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
            last.root,
            AST::Do(vec![AST::Value(Literal::Address((2, 0)))])
        );
    }
}
