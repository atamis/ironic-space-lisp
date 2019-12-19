//! Convert a [`LiftedAST`](function_lifter::LiftedAST) to a form that uses local definitions.
use crate::ast::passes::function_lifter::LASTVisitor;
use crate::ast::ASTVisitor;
use crate::ast::Def;
use crate::ast::DefVisitor;
use crate::ast::LiftedAST;
use crate::ast::AST;
use crate::data::Keyword;
use crate::data::Literal;
use crate::errors::*;
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

#[derive(Clone, Debug)]
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
        self.names.get(k).copied()
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
        let mut ctx = self.clone();
        ctx.top_level_defs = false;

        Ok(LocalAST::Let {
            defs: ctx.visit_multi_def(defs)?,
            body: Rc::new(ctx.visit(body)?),
        })
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

pub use self::visitors::*;

/// Contains visitor traits for [`LocalLiftedAST`] and related structs.
///
/// Traits include single generic visit method that dispatches on the enum
/// values, and abstract methods for each individual sub-value that take the
/// deconstructed data as parameters.
pub mod visitors {
    use super::GlobalDef;
    use super::LocalAST;
    use super::LocalDef;
    use super::LocalLiftedAST;
    use crate::data::Keyword;
    use crate::data::Literal;
    use crate::errors::*;
    use std::rc::Rc;

    /// Traverse a LocalAST, optionally producing a value alongside errors.
    pub trait LocalASTVisitor<R> {
        /// Dispatch a `LocalAST`, and add error context.
        ///
        /// This doesn't recurse itself, but relies on implementations
        /// to call `visit` again as necessary.
        fn visit(&mut self, expr: &LocalAST) -> Result<R> {
            let r = match expr {
                LocalAST::Value(l) => self.value_expr(l).context("Visiting value expr"),
                LocalAST::If { pred, then, els } => {
                    self.if_expr(pred, then, els).context("Visiting if expr")
                }
                LocalAST::Def(def) => self.def_expr(def).context("Visiting def expr"),
                LocalAST::LocalDef(def) => {
                    self.localdef_expr(def).context("Visiting localdef expr")
                }
                LocalAST::Let { defs, body } => {
                    self.let_expr(defs, body).context("Visiting let expr")
                }
                LocalAST::Do(asts) => self.do_expr(&asts).context("Visiting do expr"),
                LocalAST::GlobalVar(k) => self.globalvar_expr(k).context("Vising globalvar expr"),
                LocalAST::LocalVar(i) => self.localvar_expr(*i).context("Vising localvar expr"),
                LocalAST::Application { f, args } => self
                    .application_expr(f, args)
                    .context("Visiting application expr"),
            }?;

            Ok(r)
        }

        /// Visit multiple asts, tagging each result with indexed context, and collecting it into a result.
        fn multi_visit(&mut self, exprs: &[LocalAST]) -> Result<Vec<R>> {
            let rs: Vec<R> = exprs
                .iter()
                .enumerate()
                .map(|(i, ast)| {
                    let a = self
                        .visit(ast)
                        .context(format!("While visiting multi expression {:}", i))?;
                    Ok(a)
                })
                .collect::<Result<_>>()?;

            Ok(rs)
        }

        /// Callback for `LocalAST::Value`, passing a reference to the contained literal.
        fn value_expr(&mut self, l: &Literal) -> Result<R>;

        /// Callback for `LocalAST::If`, passing in the predicate and 2 branches.
        fn if_expr(
            &mut self,
            pred: &Rc<LocalAST>,
            then: &Rc<LocalAST>,
            els: &Rc<LocalAST>,
        ) -> Result<R>;

        /// Callback for `LocalAST::Def`, passing in a reference to the `GlobalDef`.
        fn def_expr(&mut self, def: &Rc<GlobalDef>) -> Result<R>;

        /// Callback for `LocalAST::LocalDef`, passing in a reference to the `LocalDef`.
        fn localdef_expr(&mut self, def: &Rc<LocalDef>) -> Result<R>;

        /// Callback for `LocalAST::Let`, passing in a slice of `LocalDef` and a reference to the body.
        fn let_expr(&mut self, defs: &[LocalDef], body: &Rc<LocalAST>) -> Result<R>;

        /// Callback for `LocalAST::Do`, passing in a slice of `LocalAST`.
        fn do_expr(&mut self, exprs: &[LocalAST]) -> Result<R>;

        /// Callback for `LocalAST::GlobalVar`, passing in a reference to the name.
        fn globalvar_expr(&mut self, name: &Keyword) -> Result<R>;

        /// Callback for `LocalAST::LocalVar`, passing the index directly.
        fn localvar_expr(&mut self, index: usize) -> Result<R>;

        /// Callback for `LocalAST::Application`, passing in the function and a slice of the arguments.
        fn application_expr(&mut self, f: &Rc<LocalAST>, args: &[LocalAST]) -> Result<R>;
    }

    /// Traverse one or multiple `LocalDef`s, tagging the results with context.
    pub trait LocalDefVisitor<R> {
        /// Visit multiple `LocalDef`s, collecting the result in a `Vec`.
        fn visit_multi_localdef(&mut self, defs: &[LocalDef]) -> Result<Vec<R>> {
            let rs: Vec<R> = defs
                .iter()
                .enumerate()
                .map(|(i, def)| {
                    let a = self
                        .visit_localdef(def.name, &def.value)
                        .context(format!("While parsing def #{:}", i))?;
                    Ok(a)
                })
                .collect::<Result<_>>()?;

            Ok(rs)
        }

        /// Visit a single `LocalDef`.
        ///
        /// This atuomatically destructures the `LocalDef`, and tags the result with context.
        fn visit_single_localdef(&mut self, d: &LocalDef) -> Result<R> {
            let res = self
                .visit_localdef(d.name, &d.value)
                .context(format!("While visiting localdef {:}", d.name))?;
            Ok(res)
        }

        /// Callback for a single `LocalDef`, passing in the name and value `LocalAST`.
        fn visit_localdef(&mut self, index: usize, value: &LocalAST) -> Result<R>;
    }

    /// Traverse one or multiple `GlobalDef`s, tagging the results with context.
    pub trait GlobalDefVisitor<R> {
        /// Visit multiple `GlobalDef`s, collecting the result in a `Vec`.
        fn visit_multi_globaldef(&mut self, defs: &[GlobalDef]) -> Result<Vec<R>> {
            let rs: Vec<R> = defs
                .iter()
                .enumerate()
                .map(|(i, def)| {
                    let a = self
                        .visit_globaldef(&def.name, &def.value)
                        .context(format!("While parsing def #{:}", i))?;
                    Ok(a)
                })
                .collect::<Result<_>>()?;

            Ok(rs)
        }

        /// Visit a single `GlobalDef`.
        ///
        /// This atuomatically destructures the `GlobalDef`, and tags the result with context.
        fn visit_single_globaldef(&mut self, d: &GlobalDef) -> Result<R> {
            let res = self
                .visit_globaldef(&d.name, &d.value)
                .context(format!("While visiting globaldef {:}", d.name))?;
            Ok(res)
        }

        /// Callback for a single `GlobalDef`, passing in the name and value `LocalAST`.
        fn visit_globaldef(&mut self, name: &Keyword, value: &LocalAST) -> Result<R>;
    }

    /// Traverse a `LocalLiftedAST`, with error context tagging.
    pub trait LLASTVisitor<R> {
        /// Visit all the functions in a `LocalLiftedAST`, tagging them with their index.
        fn llast_visit(&mut self, llast: &LocalLiftedAST) -> Result<Vec<R>> {
            let entry = llast.entry;

            let rs = llast
                .functions
                .iter()
                .enumerate()
                .map(|(idx, func)| {
                    let res = self
                        .visit_local_function(&func.args, &func.body, idx == entry)
                        .context(format!("While visiting function {:}", idx))?;

                    Ok(res)
                })
                .collect::<Result<_>>()
                .context("While visiting LocalLiftedAST")?;

            Ok(rs)
        }

        /// Visit a local function, passing in references to the arguments, body, and whether this function is the entry.
        fn visit_local_function(
            &mut self,
            args: &[Keyword],
            body: &Rc<LocalAST>,
            entry: bool,
        ) -> Result<R>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::ast::passes::function_lifter;
    use crate::ast::passes::internal_macro;
    use crate::ast::passes::unique;
    use crate::parser;

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
