use ast::passes::function_lifter::LASTVisitor;
use ast::ASTVisitor;
use ast::Def;
use ast::LiftedAST;
use ast::AST;
use data::Keyword;
use data::Literal;
use errors::*;
use im::hashmap;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum LocalAST {
    Value(Literal),
    If {
        pred: Rc<LocalAST>,
        then: Rc<LocalAST>,
        els: Rc<LocalAST>,
    },
    Def(Rc<Def>),
    LocalDef(Rc<LocalDef>),
    Let {
        defs: Vec<LocalDef>,
        body: Rc<LocalAST>,
    },
    Do(Vec<LocalAST>),
    Lambda {
        args: Vec<Keyword>,
        body: Rc<LocalAST>,
    },
    Local(usize),
    Var(Keyword),
    Application {
        f: Rc<LocalAST>,
        args: Vec<LocalAST>,
    },
}

#[derive(Debug, PartialEq)]
pub struct LocalDef {
    pub name: usize,
    pub value: LocalAST,
}

pub struct LocalFunction {
    pub args: Vec<Keyword>,
    pub body: Rc<LocalAST>,
}

pub struct LocalLiftedAST {
    pub functions: Vec<LocalFunction>,
    pub entry: usize,
}

pub fn pass(last: &LiftedAST) -> Result<LocalLiftedAST> {
    let mut l = Lifter {};

    let fns = l.last_visit(last)?;

    Ok(LocalLiftedAST {
        functions: fns,
        entry: last.entry,
    })
}

struct Lifter;

impl LASTVisitor<LocalFunction> for Lifter {
    fn ast_function(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<LocalFunction> {
        Ok(LocalFunction {
            args: vec![],
            body: Rc::new(LocalAST::Value(1.into())),
        })
    }
}

struct Localizer {
    names: HashMap<Keyword, usize>,
    index: usize,
}

impl Localizer {
    pub fn new() -> Localizer {
        Localizer {
            names: HashMap::new(),
            index: 0,
        }
    }

    pub fn check_keyword(&mut self, k: &Keyword) -> usize {
        if let Some(i) = self.names.get(k) {
            return *i;
        }
        let i = self.index;
        self.index += 1;
        self.names.insert(k.to_string(), i);
        i
    }
}

/*impl ASTVisitor<LocalAST> for Localizer {
    fn value_expr(&mut self, l: &Literal) -> Result<R>;

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<R>;

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<R>;

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<R>;

    fn do_expr(&mut self, exprs: &[AST]) -> Result<R>;

    fn lambda_expr(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<R>;

    #[allow(clippy::ptr_arg)]
    fn var_expr(&mut self, k: &Keyword) -> Result<R>;

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<R>;
}*/

#[cfg(tests)]
mod tests {
    #[test]
    fn test_localizer() {
        let mut l = Localizer::new();
        let i = l.check_keyword("test");

        assert_eq(i, l.check_keyword("test"));

        let i2 = l.check_keyword("test2");

        assert_ne!(i1, i2);
    }
}
