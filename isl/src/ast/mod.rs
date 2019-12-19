//! [`AST`](ast::AST) definition, AST parser, and the [`ASTVisitor`](ast::ASTVisitor) utility trait.
//!
//! This AST specifies some special forms. The construction of the
//! [`Literal`](data::Literal) values makes matching them properly difficulty,
//! and providing meaningful errors harder. This simplifies the
//! error reporting, an offers an easy way of traversing the AST,
//! the [`ASTVisitor`](ast::ASTVisitor) trait.

use im::vector::Vector;
use std::rc::Rc;

use crate::data;
use crate::data::Literal;
use crate::data::Symbol;
use crate::env;
use crate::errors::*;

pub mod passes;
use self::passes::function_lifter;
pub use self::passes::function_lifter::LiftedAST;
use self::passes::internal_macro;
use self::passes::local;
use self::passes::unbound;

/// Representation of Lisp code in terms of special forms and applications.
#[derive(Debug, PartialEq)]
pub enum AST {
    /// A literal value.
    Value(Literal),
    /// An `if` expression.
    If {
        /// The predicate.
        pred: Rc<AST>,
        /// The true branch.
        then: Rc<AST>,
        /// The false branch.
        ///
        /// This would be `else`, but that's a reserved Symbol.
        els: Rc<AST>,
    },
    /// A single def expression, defining a Symbol based on a body.
    Def(Rc<Def>),
    /// A let expression, allowing for local bindings in a body.
    Let {
        /// The local defs
        defs: Vec<Def>,
        /// The body
        body: Rc<AST>,
    },
    /// Expression for executing multiple expressions, evaluating to the value of the last expression.
    Do(Vec<AST>),
    /// Lambda expression representing a function, having args and a body.
    Lambda {
        /// A list of the argument names.
        args: Vec<Symbol>,
        /// The body.
        body: Rc<AST>,
    },
    /// A variable ref expression.
    Var(Symbol),
    /// A function application expression.
    Application {
        /// The function expression.
        f: Rc<AST>,
        /// The arguments to the function.
        args: Vec<AST>,
    },
}

/// Represents a "definition", either a local binding or a top level definition.
#[derive(Debug, PartialEq)]
pub struct Def {
    /// The name of the `Def`.
    pub name: Symbol,
    /// The [`AST`] representing the body of the `Def`.
    pub value: AST,
}

/// Parse several [`Literal`]s into a [`LiftedAST`].
pub fn ast(lits: &[data::Literal], e: &env::Env) -> Result<local::LocalLiftedAST> {
    let last = {
        let ast = parse_multi(&lits).context("Multiparsing literals")?;
        let ast = internal_macro::pass(&ast).context("Expanding internal macros")?;

        unbound::pass(&ast, e).context("Checking unbound variables")?;

        let last = function_lifter::lift_functions(&ast).context("Lifting functions")?;

        local::pass(&last).context("Locals pass")
    }
    .context("While parsing multiple literals")?;

    Ok(last)
}

/// Visit single or multiple `Def`s easily and with nice error tagging.
pub trait DefVisitor<R> {
    /// Visit multiple `Def`s, collecting the result in a `Vec`.
    fn visit_multi_def(&mut self, defs: &[Def]) -> Result<Vec<R>> {
        let rs: Vec<R> = defs
            .iter()
            .enumerate()
            .map(|(i, def)| {
                let a = self
                    .visit_def(&def.name, &def.value)
                    .context(format!("While parsing def #{:}", i))?;
                Ok(a)
            })
            .collect::<Result<_>>()?;

        Ok(rs)
    }

    /// Visit a single `Def`.
    ///
    /// This atuomatically destructures the `Def`, and tags the result with context.
    fn visit_single_def(&mut self, d: &Def) -> Result<R> {
        let res = self
            .visit_def(&d.name, &d.value)
            .context(format!("While visiting def {:}", d.name))?;
        Ok(res)
    }

    /// Callback for a single `Def`, passing in the name and value `AST`.
    fn visit_def(&mut self, name: &str, value: &AST) -> Result<R>;
}

/// Traverse an AST, optionally producing a value alongside errors.
pub trait ASTVisitor<R> {
    /// Dispatch an `AST`, and add error context.
    ///
    /// This doesn't recurse itself, but relies on implementations
    /// to call `visit` again as necessary.
    fn visit(&mut self, a: &AST) -> Result<R> {
        let r = match a {
            AST::Value(l) => self.value_expr(l).context("Visiting value expr"),
            AST::If { pred, then, els } => {
                self.if_expr(pred, then, els).context("Visiting if expr")
            }
            AST::Def(def) => self.def_expr(def).context("Visiting def expr"),
            AST::Let { defs, body } => self.let_expr(defs, body).context("Visiting let expr"),
            AST::Do(asts) => self.do_expr(asts).context("Visiting do expr"),
            AST::Lambda { args, body } => {
                self.lambda_expr(args, body).context("Visiting lambda expr")
            }
            AST::Var(k) => self.var_expr(k).context("Vising var expr"),
            AST::Application { f, args } => self
                .application_expr(f, args)
                .context("Visiting application expr"),
        }?;

        Ok(r)
    }

    /// Visit multiple asts, tagging each result with indexed context, and collecting it into a result.
    fn multi_visit(&mut self, asts: &[AST]) -> Result<Vec<R>> {
        let rs: Vec<R> = asts
            .iter()
            .enumerate()
            .map(|(i, ast)| {
                let a = self
                    .visit(ast)
                    .context(format!("While parsing multi expression {:}", i))?;
                Ok(a)
            })
            .collect::<Result<_>>()?;

        Ok(rs)
    }

    /// Callback for `AST::Value`, passing in a reference to the container literal.
    fn value_expr(&mut self, l: &Literal) -> Result<R>;

    /// Callback for `AST::If`, passing in the predicate and both branches.
    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<R>;

    /// Callback for `AST::Def`, passing in the `Def`.
    fn def_expr(&mut self, def: &Rc<Def>) -> Result<R>;

    /// Callback for `AST::Let`, passing in a slice of the `Def`s and the body `AST`.
    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<R>;

    /// Callback for `AST::Do`, passing in a slice of the `AST`s.
    fn do_expr(&mut self, exprs: &[AST]) -> Result<R>;

    /// Callback for `AST::Lambda`, passing in a slice of the arguments and the body.
    fn lambda_expr(&mut self, args: &[Symbol], body: &Rc<AST>) -> Result<R>;

    /// Callback for `AST::Var`, passing in the name.
    #[allow(clippy::ptr_arg)]
    fn var_expr(&mut self, k: &Symbol) -> Result<R>;

    /// Callback for `AST::Application`, passing in the function, and a slice of the arguments.
    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<R>;
}

/// Convert a variable number of ASTs to a single AST.
///
/// With 1 AST, return the AST
///
/// With 0 ASTs, return an AST that returns false
///
/// With any other number, return the ASTs wrapped in a do
/// expression, which evaluates to the last expression.
fn wrap_do(mut asts: Vec<AST>) -> AST {
    match asts.len() {
        1 => asts.remove(0),
        0 => AST::Value(false.into()),
        _ => AST::Do(asts),
    }
}

/// Parse potentially multiple exprs, returning exactly 1 AST.
///
/// Returns either the single [`AST`] contained in the literals,
/// or multiple [`AST`]s from the literals, wrapped in a [`AST::Do`] expr.
pub fn parse_multi(exprs: &[Literal]) -> Result<AST> {
    let asts: Vec<AST> = exprs
        .iter()
        .enumerate()
        .map(|(i, lit)| {
            let a = parse(&lit).context(format!("While parsing literal #{:}", i))?;
            Ok(a)
        })
        .collect::<Result<_>>()?;

    Ok(wrap_do(asts))
}

/// Parse raw sexprs ([`Literal`]) into an AST.
pub fn parse(e: &Literal) -> Result<AST> {
    match e {
        Literal::List(ref vec) => {
            match vec.len() {
                0 => Err(err_msg("empty list not valid")), // TODO
                1 => parse_compound(&vec[0], &Vector::new()),
                _ => {
                    let (first, rest) = vec.clone().split_at(1);
                    parse_compound(&first[0], &rest)
                }
            }
        }
        Literal::Symbol(k) => Ok(AST::Var(k.clone())),
        Literal::Boolean(_) => Ok(AST::Value(e.clone())),
        Literal::Number(_) => Ok(AST::Value(e.clone())),
        Literal::Address(_) => Err(err_msg("Address literals not supported")),
        Literal::Closure(_, _) => Err(err_msg("Closure literals not supported")),
        Literal::Pid(_) => Err(err_msg("Pid literals are not supported")),
    }
}

fn parse_compound(first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    let r = if let Literal::Symbol(s) = first {
        match s.as_ref() {
            "if" => parse_if(first, rest).context("Parsing let expr"),
            "def" => parse_def_expr(first, rest).context("Parsing def expr"),
            "let" => parse_let(first, rest).context("Parsing let expr"),
            "do" => parse_do(first, rest).context("Parsing do expr"),
            "lambda" => parse_lambda(first, rest).context("Parsing lambda expr"),
            "fn" => parse_lambda(first, rest).context("Parsing fn lambda expr"),
            "quote" => parse_quote(first, rest).context("Parsing quoted expr"),
            "quasiquote" => parse_quasiquote(first, rest).context("Parsing quasiquoted expr"),
            _ => parse_application(first, rest).context("Parsing application expr"),
        }
    } else {
        parse_application(first, rest).context("Parsing application expr")
    }?;

    Ok(r)
}

fn parse_def_single(v: &Vector<Literal>) -> Result<Def> {
    if v.len() > 2 {
        return Err(err_msg("Excessive items after def"));
    }

    match parse_def_partial(v) {
        Ok(d) => Ok(d),
        Err(e) => Err(e),
    }
}

fn parse_def_partial(v: &Vector<Literal>) -> Result<Def> {
    if v.len() < 2 {
        return Err(err_msg("Insufficient terms for def"));
    }

    let name;

    if let Literal::Symbol(ref s) = v[0] {
        name = s.clone();
    } else {
        return Err(err_msg("first term of def must be Symbol"));
    }

    let v = parse(&v[1]).context("Second term of def must be valid AST")?;

    Ok(Def { name, value: v })
}

fn parse_if(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    if rest.len() != 3 {
        return Err(err_msg("malformed if expr, (if pred then else)"));
    }

    let mut asts: Vec<Rc<AST>> = rest
        .iter()
        .map(|l| parse(l))
        .collect::<Result<Vec<AST>>>()? // make sure there are no parse errors
        .into_iter()
        .map(Rc::new)
        .collect();

    // These shouldn't fail, based on the length test above.
    let els = asts
        .pop()
        .ok_or_else(|| err_msg("If requires else clause"))?;
    let then = asts
        .pop()
        .ok_or_else(|| err_msg("If requires then clause"))?;
    let pred = asts.pop().ok_or_else(|| err_msg("If requires predicate"))?;

    Ok(AST::If { pred, then, els })
}

fn parse_def_expr(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    let def = parse_def_single(rest)?;
    Ok(AST::Def(Rc::new(def)))
}

fn parse_let(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    let def_literals = rest
        .get(0)
        .ok_or_else(|| err_msg("let requires def list as first term (let (defs+) body)"))?
        .ensure_list()
        .context("Parsing list of defs")?;

    let body_literals = rest.skip(1);

    if def_literals.len() % 2 != 0 {
        return Err(err_msg("in let, def list must be even"));
    }

    let body_asts = body_literals
        .iter()
        .map(parse)
        .collect::<Result<_>>()
        .context("While parsing body of let")?;

    let body = Rc::new(wrap_do(body_asts));

    let mut defs = Vec::with_capacity(def_literals.len() / 2);

    let mut def_literals = def_literals;

    // TODO: currently can't report def index
    // TODO: unfuck
    while !def_literals.is_empty() {
        defs.push(parse_def_partial(&def_literals).context("Parsing defs in let")?);

        if 2 > def_literals.len() {
            return Err(err_msg("Error slicing defs, not enough def terms"));
        }
        if 2 == def_literals.len() {
            break;
        }
        def_literals = def_literals.split_off(2);
    }

    Ok(AST::Let { defs, body })
}

fn parse_do(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    Ok(AST::Do(rest.iter().map(parse).collect::<Result<_>>()?))
}

fn parse_lambda(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    let args = rest
        .get(0)
        .ok_or_else(|| err_msg("lambda requires an argument list, (lambda (args*) body)"))?
        .ensure_list()?
        .iter()
        .map(Literal::ensure_symbol)
        .collect::<Result<_>>()?;

    let body = rest
        .skip(1)
        .iter()
        .map(parse)
        .collect::<Result<_>>()
        .context("lambda requires body, (lambda (args*) body)")?;
    let body = Rc::new(wrap_do(body));

    Ok(AST::Lambda { args, body })
}

fn parse_quote(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    if rest.len() > 1 {
        Err(err_msg(
            "Inexplicable additional arguments to quoted expression, (quote lit)",
        ))
    } else {
        Ok(AST::Value(rest[0].clone()))
    }
}

fn parse_quasiquote(_first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    if rest.len() != 1 {
        return Err(err_msg(
            "Additional arguments to quasiquote, (quasiquote lit)",
        ));
    }

    Ok(dynamic_quasiquote(&rest[0]).context("While parsing quasiquote")?)
}

fn dynamic_quasiquote(a: &Literal) -> Result<AST> {
    let uq = Literal::Symbol("unquote".to_string());
    // Is dynamic structure necessary
    if a.contains(&uq) {
        if let Literal::List(l) = a {
            if l.len() == 2 && l[0] == uq {
                // Parse unquoted stuff. This should remove the unquote "call"
                let tree = parse(&l[1]).context("While parsing unquote")?;
                return Ok(tree);
            }

            // Dynamically build the list at runtime.
            return Ok(AST::Application {
                f: Rc::new(AST::Var("list".to_string())),
                args: l.iter().map(dynamic_quasiquote).collect::<Result<_>>()?,
            });
        }
    }

    // No? act like (quote x)
    Ok(AST::Value(a.clone()))
}

fn parse_application(first: &Literal, rest: &Vector<Literal>) -> Result<AST> {
    let f = Rc::new(parse(first).context("Function AST in application")?);

    let args = rest
        .iter()
        .map(parse)
        .collect::<Result<_>>()
        .context("Arguments to application")?;

    Ok(AST::Application { f, args })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::data::Literal;
    use crate::parser::Parser;
    use std::rc::Rc;

    fn p(s: &str) -> Result<Vec<Literal>> {
        let p = Parser::new();
        p.parse(s)
    }

    // parse_string
    fn ps(s: &str) -> Result<AST> {
        parse(&p(s).unwrap()[0])
    }

    #[test]
    fn test_value() {
        assert_eq!(ps("1").unwrap(), AST::Value(Literal::Number(1)));
        assert_eq!(
            parse(&Literal::Boolean(true)).unwrap(),
            AST::Value(Literal::Boolean(true))
        );
        assert!(parse(&Literal::Address((0, 0))).is_err());
    }

    #[test]
    fn test_if() {
        assert_eq!(
            ps("(if 0 0 0)").unwrap(),
            AST::If {
                pred: Rc::new(ps("0").unwrap()),
                then: Rc::new(ps("0").unwrap()),
                els: Rc::new(ps("0").unwrap()),
            }
        );

        assert!(ps("(if)").is_err());
        assert!(ps("(if 0)").is_err());
        assert!(ps("(if 0 0)").is_err());
    }

    #[test]
    fn test_def_parital() {
        let p1 = p("test 0").unwrap().into_iter().collect();

        assert_eq!(
            parse_def_partial(&p1).unwrap(),
            Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }
        );

        let p2 = p("0 0").unwrap().into_iter().collect();

        assert!(parse_def_partial(&p2).is_err());

        let p3 = p("test 0 asdf").unwrap().into_iter().collect();

        assert_eq!(
            parse_def_partial(&p3).unwrap(),
            Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }
        );
    }

    #[test]
    fn test_def_single() {
        // Mostly copied from test_def_partial
        let p1 = p("test 0").unwrap().into_iter().collect();

        assert_eq!(
            parse_def_single(&p1).unwrap(),
            Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }
        );

        let p2: Vector<Literal> = p("0 0").unwrap().into_iter().collect();

        assert!(parse_def_single(&p2).is_err());

        let p3: Vector<Literal> = p("test 0 asdf").unwrap().into_iter().collect();

        assert!(parse_def_single(&p3).is_err());
    }

    #[test]
    fn test_def() {
        let p1 = ps("(def test 0)").unwrap();

        assert_eq!(
            p1,
            AST::Def(Rc::new(Def {
                name: "test".to_string(),
                value: AST::Value(Literal::Number(0))
            }))
        );

        // Check errors are passed, assume other errors work
        assert!(ps("(def test)").is_err());
    }

    #[test]
    fn test_let() {
        let p1 = ps("(let (test 0) 0)").unwrap();

        assert_eq!(
            p1,
            AST::Let {
                defs: vec![Def {
                    name: "test".to_string(),
                    value: AST::Value(Literal::Number(0))
                }],
                body: Rc::new(AST::Value(Literal::Number(0)))
            }
        );

        let p2 = ps("(let (test 0 asdf 0) 0)").unwrap();

        assert_eq!(
            p2,
            AST::Let {
                defs: vec![
                    Def {
                        name: "test".to_string(),
                        value: AST::Value(Literal::Number(0))
                    },
                    Def {
                        name: "asdf".to_string(),
                        value: AST::Value(Literal::Number(0))
                    },
                ],
                body: Rc::new(AST::Value(Literal::Number(0)))
            }
        );

        let p3 = ps("(let (test 0 asdf) 0)");

        assert!(p3.is_err());

        let p4 = ps("(let (test 0))").unwrap();

        assert_eq!(
            p4,
            AST::Let {
                defs: vec![Def {
                    name: "test".to_string(),
                    value: AST::Value(Literal::Number(0))
                },],
                body: Rc::new(AST::Value(false.into()))
            }
        );

        let p5 = ps("(let () 0)").unwrap();

        assert_eq!(
            p5,
            AST::Let {
                defs: vec![],
                body: Rc::new(AST::Value(0.into()))
            }
        );
    }

    #[test]
    fn test_var() {
        let p1 = ps("test").unwrap();

        assert_eq!(p1, AST::Var("test".to_string()));

        let p2 = ps("asdf1234").unwrap();

        assert_eq!(p2, AST::Var("asdf1234".to_string()));

        let p3 = ps("+").unwrap();

        assert_eq!(p3, AST::Var("+".to_string()));
    }

    #[test]
    fn test_do() {
        let p1 = ps("(do 0 0 0 0)").unwrap();

        assert_eq!(
            p1,
            AST::Do(vec![
                AST::Value(Literal::Number(0)),
                AST::Value(Literal::Number(0)),
                AST::Value(Literal::Number(0)),
                AST::Value(Literal::Number(0)),
            ])
        );

        let p2 = ps("(do)").unwrap();

        assert_eq!(p2, AST::Do(vec![]))
    }

    #[test]
    fn test_lambda() {
        let p1 = ps("(lambda (test) 0)").unwrap();

        assert_eq!(
            p1,
            AST::Lambda {
                args: vec!["test".to_string()],
                body: Rc::new(AST::Value(Literal::Number(0))),
            }
        );

        let p2 = ps("(lambda () 0)").unwrap();

        assert_eq!(
            p2,
            AST::Lambda {
                args: vec![],
                body: Rc::new(AST::Value(Literal::Number(0))),
            }
        );

        let p3 = ps("(lambda ())").unwrap();

        assert_eq!(
            p3,
            AST::Lambda {
                args: vec![],
                body: Rc::new(AST::Value(false.into())),
            }
        );

        assert!(ps("(lambda 0)").is_err());
    }

    #[test]
    fn test_application() {
        let p1 = ps("(+ 0 0 0)").unwrap();

        assert_eq!(
            p1,
            AST::Application {
                f: Rc::new(AST::Var("+".to_string())),
                args: vec![
                    AST::Value(Literal::Number(0)),
                    AST::Value(Literal::Number(0)),
                    AST::Value(Literal::Number(0)),
                ]
            }
        );

        let p2 = ps("(+)").unwrap();

        assert_eq!(
            p2,
            AST::Application {
                f: Rc::new(AST::Var("+".to_string())),
                args: vec![],
            }
        )
    }

    #[test]
    fn test_quoted() {
        let p1 = ps("'1").unwrap();

        assert_eq!(p1, AST::Value(Literal::Number(1)));
    }

    #[test]
    fn test_quasiquote() {
        assert_eq!(ps("`1").unwrap(), AST::Value(Literal::Number(1)));

        assert_eq!(
            ps("`(test asdf ,(+ 1 2 3))").unwrap(),
            ps("(list 'test 'asdf (+ 1 2 3))").unwrap()
        );

        assert_eq!(
            ps("`(test asdf ,x)").unwrap(),
            ps("(list 'test 'asdf x)").unwrap()
        );
    }
}
