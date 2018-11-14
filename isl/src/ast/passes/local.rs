//! Convert a [`LiftedAST`](function_lifter::LiftedAST) to a form that uses local definitions.
use ast::passes::function_lifter::LASTVisitor;
use ast::ASTVisitor;
use ast::Def;
use ast::DefVisitor;
use ast::LiftedAST;
use ast::AST;
use data::Keyword;
use data::Literal;
use errors::*;
use std::collections::HashMap;
use std::rc::Rc;

/// An [`AST`] that has local and global defs.
#[allow(missing_docs)]
#[derive(Debug, PartialEq)]
pub enum LocalAST {
    Value(Literal),
    If {
        pred: Rc<LocalAST>,
        then: Rc<LocalAST>,
        els: Rc<LocalAST>,
    },
    Def(Rc<GlobalDef>),
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
    GlobalVar(Keyword),
    LocalVar(usize),
    Application {
        f: Rc<LocalAST>,
        args: Vec<LocalAST>,
    },
}

/// A local def relating an index with a [`LocalAST`].
#[derive(Debug, PartialEq)]
pub struct LocalDef {
    /// The index of this local def.
    pub name: usize,
    /// The [`LocalAST`] body of this local def.
    pub value: LocalAST,
}

/// A global def relating a keyword name to a [`LocalAST`].
#[derive(Debug, PartialEq)]
pub struct GlobalDef {
    /// The name of this global def.
    pub name: Keyword,
    /// The [`LocalAST`] body of this global def.
    pub value: LocalAST,
}

/// A function with local definitions, where the body is [`LocalAST`].
#[derive(Debug)]
pub struct LocalFunction {
    /// This functions argument names.
    pub args: Vec<Keyword>,
    /// The body of the function
    pub body: Rc<LocalAST>,
}

/// A collection of [`LocalFunction`] with 1 entry point.
#[derive(Debug)]
pub struct LocalLiftedAST {
    /// `Vec` of [`LocalFunction`].
    pub functions: Vec<LocalFunction>,
    /// Index of the entry point.
    pub entry: usize,
}

/// Do the pass. See [`local`](super::local).
pub fn pass(last: &LiftedAST) -> Result<LocalLiftedAST> {
    let mut l = Localizer::new();

    let fns = l.last_visit(last)?;

    Ok(LocalLiftedAST {
        functions: fns,
        entry: last.entry,
    })
}

// Private Implmentation

struct Localizer;

impl Localizer {
    pub fn new() -> Localizer {
        Localizer {}
    }
}

impl LASTVisitor<LocalFunction> for Localizer {
    fn ast_function(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<LocalFunction> {
        let mut l = FunctionLocalizer::new(args, false);

        Ok(LocalFunction {
            args: args.to_vec(),
            body: Rc::new(l.visit(body)?),
        })
    }

    fn ast_function_entry(&mut self, args: &[Keyword], body: &Rc<AST>) -> Result<LocalFunction> {
        let mut l = FunctionLocalizer::new(args, true);

        Ok(LocalFunction {
            args: args.to_vec(),
            body: Rc::new(l.visit(body)?),
        })
    }
}

#[derive(Clone)]
struct FunctionLocalizer {
    names: HashMap<Keyword, usize>,
    index: usize,
    top_level_defs: bool,
}

impl FunctionLocalizer {
    fn new(args: &[Keyword], top_level_defs: bool) -> FunctionLocalizer {
        let mut l = FunctionLocalizer {
            names: HashMap::new(),
            index: 0,
            top_level_defs,
        };

        for k in args {
            l.check_keyword(k);
        }

        l
    }

    pub fn check_keyword(&mut self, k: &str) -> usize {
        if let Some(i) = self.names.get(k) {
            return *i;
        }
        let i = self.index;
        self.index += 1;
        self.names.insert(k.to_string(), i);
        i
    }

    pub fn get_keyword(&mut self, k: &str) -> Option<usize> {
        self.names.get(k).map(|i| *i)
    }
}

impl DefVisitor<LocalDef> for FunctionLocalizer {
    fn visit_def(&mut self, name: &str, value: &AST) -> Result<LocalDef> {
        Ok(LocalDef {
            name: self.check_keyword(&name),
            value: self.visit(value)?,
        })
    }
}

impl ASTVisitor<LocalAST> for FunctionLocalizer {
    fn value_expr(&mut self, l: &Literal) -> Result<LocalAST> {
        Ok(LocalAST::Value(l.clone()))
    }

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<LocalAST> {
        Ok(LocalAST::If {
            pred: Rc::new(self.visit(pred)?),
            then: Rc::new(self.visit(then)?),
            els: Rc::new(self.visit(els)?),
        })
    }

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<LocalAST> {
        if self.top_level_defs {
            let def = GlobalDef {
                name: def.name.to_string(),
                value: self.visit(&def.value)?,
            };

            Ok(LocalAST::Def(Rc::new(def)))
        } else {
            Ok(LocalAST::LocalDef(Rc::new(self.visit_single_def(def)?)))
        }
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<LocalAST> {
        // Save TLD, then disable them inside the let, then restore it.
        let tld = self.top_level_defs;
        self.top_level_defs = false;

        let ans = Ok(LocalAST::Let {
            defs: self.visit_multi_def(defs)?,
            body: Rc::new(self.visit(body)?),
        });

        self.top_level_defs = tld;

        ans
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<LocalAST> {
        Ok(LocalAST::Do(self.multi_visit(exprs)?))
    }

    fn lambda_expr(&mut self, _args: &[Keyword], _body: &Rc<AST>) -> Result<LocalAST> {
        Err(err_msg("local pass does not support lambda"))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<LocalAST> {
        Ok(match self.get_keyword(k) {
            Some(i) => LocalAST::LocalVar(i),
            None => LocalAST::GlobalVar(k.to_string()),
        })
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<LocalAST> {
        Ok(LocalAST::Application {
            f: Rc::new(self.visit(f)?),
            args: self.multi_visit(args)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast;
    use ast::passes::function_lifter;
    use ast::passes::internal_macro;
    use ast::passes::unique;
    use parser;

    fn do_last(s: &str) -> LiftedAST {
        let ast = ast::parse_multi(&parser::parse(s).unwrap()).unwrap();

        let ast = internal_macro::pass(&ast).unwrap();

        let last = function_lifter::lift_functions(&ast).unwrap();

        unique::pass(&last).unwrap()
    }

    fn do_pass(s: &str) -> Result<LocalLiftedAST> {
        let last = do_last(s);

        pass(&last)
    }

    #[test]
    fn test_localizer() {
        let mut l = FunctionLocalizer::new(&vec![], true);
        let i1 = l.check_keyword("test");

        assert_eq!(i1, l.check_keyword("test"));

        let i2 = l.check_keyword("test2");

        assert_ne!(i1, i2);
    }

    #[test]
    fn test_local_args() {
        let llast = do_pass("(def x 1) (def f (lambda (n) (+ n x)))").unwrap();
        println!("{:?}", llast);

        // 0 is always the entry, so 1 is the first non-entry function.
        let lfn = &llast.functions[1];

        if let LocalAST::Application { ref f, ref args } = *lfn.body {
            assert_eq!(**f, LocalAST::GlobalVar("+".to_string()));

            let localarg = &args[0];
            let globalarg = &args[1];

            // Has to be localvar 0 because it's the first argument.
            assert_eq!(*localarg, LocalAST::LocalVar(0));

            assert_eq!(*globalarg, LocalAST::GlobalVar("x".to_string()));
        } else {
            panic!();
        }

        //assert!(false);
    }

}
