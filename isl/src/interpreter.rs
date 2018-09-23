use std::rc::Rc;

use ast::passes::function_lifter;
use ast::passes::function_lifter::LiftedAST;
use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data::Address;
use data::Keyword;
use data::Literal;
use environment::Env;
use errors::*;
use syscall;
use vm;

#[derive(Debug)]
pub struct Interpreter {
    sys: syscall::SyscallRegistry,
    pub global: Env,
    last: LiftedAST,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a binding between an interpreter and its global state and a local environment.
struct Context<'a, 'b> {
    terp: &'a Interpreter,
    env: &'b mut Env,
}

impl<'a, 'b> Context<'a, 'b> {
    pub fn new(terp: &'a Interpreter, env: &'b mut Env) -> Context<'a, 'b> {
        Context { terp, env }
    }

    pub fn with_new_env(&self, env: &'b mut Env) -> Context<'a, 'b> {
        Context::new(self.terp, env)
    }
}

impl<'a, 'b> ASTVisitor<Literal> for Context<'a, 'b> {
    fn value_expr(&mut self, l: &Literal) -> Result<Literal> {
        Ok(l.clone())
    }

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<Literal> {
        let pv = self.visit(pred).context("Evaluating predicate for if")?;

        if pv.truthy() {
            Ok(self.visit(then).context("Evaluating then for if")?)
        } else {
            Ok(self.visit(els).context("Evaluating else for if")?)
        }
    }

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<Literal> {
        let res = put_def(self, def).context("Evaluating def")?;

        Ok(res)
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<Literal> {
        let mut let_env = self.env.clone();
        let mut let_context = self.with_new_env(&mut let_env);

        for d in defs {
            // TODO binding index
            put_def(&mut let_context, d).context("Evalutaing bindings for let")?;
        }

        let body_val = let_context.visit(body).context("Evaluting let body")?;

        Ok(body_val)
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<Literal> {
        let mut vals: Vec<Literal> = self
            .multi_visit(exprs)
            .context("Evaluating do sub-expressions")?;
        Ok(vals
            .pop()
            .ok_or_else(|| err_msg("do expressions can't be empty"))?)
    }

    fn lambda_expr(&mut self, _args: &[Keyword], _body: &Rc<AST>) -> Result<Literal> {
        Err(err_msg("Not implemented"))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<Literal> {
        let r = self
            .env
            .get(k)
            .ok_or_else(|| format_err!("While accessing var {:} in env {:?}", k, self.env))?;

        Ok((**r).clone())
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<Literal> {
        let f_v = self.visit(f)?;
        let f_addr = f_v.ensure_address_flexible()?;

        let vals = self
            .multi_visit(args)
            .context("Evaluating function arguments")?;

        self.terp.call_fn_addr(f_addr, vals)
    }
}

fn put_def(ctx: &mut Context, def: &Def) -> Result<Literal> {
    let res = ctx.visit(&def.value).context(format_err!(
        "While evaluating def value for {:}",
        def.name.clone()
    ))?;
    ctx.env.insert(def.name.clone(), Rc::new(res.clone()));
    Ok(res)
}

impl Interpreter {
    /// New interpreter with the default syscall bindings.
    pub fn new() -> Interpreter {
        let (sys, global) = Interpreter::default_environment();

        Interpreter {
            sys,
            global,
            last: function_lifter::lift_functions(&AST::Value(false.into())).unwrap(),
        }
    }

    /// Create interpreter with a given LiftedAST. Also invokes the entry function and throws away result.
    pub fn with_last(last: &LiftedAST) -> Result<Interpreter> {
        let (sys, global) = Interpreter::default_environment();

        let mut i = Interpreter {
            sys,
            global,
            last: (*last).clone(),
        };

        i.call_addr_global((last.entry, 0))
            .context("While interpreting the entry function")?;

        Ok(i)
    }

    /// Stick all the syscalls into an Env and registry.
    fn default_environment() -> (syscall::SyscallRegistry, Env) {
        let mut sys = syscall::SyscallRegistry::new();
        let mut global = Env::new();

        vm::ingest_environment(&mut sys, &mut global, &syscall::list::Factory::new());
        vm::ingest_environment(&mut sys, &mut global, &syscall::util::Factory::new());
        vm::ingest_environment(&mut sys, &mut global, &syscall::math::Factory::new());

        (sys, global)
    }

    /// Call a function or syscall by address, with the given arguments. Returns the result or an error.
    pub fn call_fn_addr(&self, addr: Address, mut args: Vec<Literal>) -> Result<Literal> {
        // Check function registry
        let astfn = self.last.fr.lookup(addr);

        if astfn.is_none() {
            return self.invoke_syscall(addr, args);
        }

        let astfn = astfn.unwrap();

        if astfn.arity() != args.len() {
            return Err(format_err!(
                "Error calling function {:?}, expected {:} args, got {:} args",
                addr,
                astfn.arity(),
                args.len()
            ));
        }

        let mut arg_binding = self.global.clone();

        for (name, arg) in astfn.args.iter().cloned().zip(args) {
            arg_binding.insert(name, Rc::new(arg));
        }

        let mut fn_ctx = Context::new(self, &mut arg_binding);

        Ok(fn_ctx
            .visit(&astfn.body)
            .context("While executing body of function")?)
    }

    fn invoke_syscall(&self, addr: Address, mut args: Vec<Literal>) -> Result<Literal> {
        // check syscall registry
        match self.sys.lookup(addr) {
            Some(scall) => {
                // don't even deal
                if let syscall::Syscall::Stack(_) = scall {
                    return Err(format_err!(
                        "Interpreter can't call stack syscalls, found at {:?}",
                        addr
                    ));
                }

                let sysarity = scall.arity().unwrap();

                if sysarity != args.len() {
                    return Err(format_err!(
                        "Error calling function {:?}, expected {:} args, got {:} args",
                        addr,
                        sysarity,
                        args.len()
                    ));
                }

                // Have to call these functions by value
                return match scall {
                    // Use unreachable instead of wildcard to we get warned when we
                    // add new types of syscalls
                    syscall::Syscall::Stack(_) => unreachable!(),
                    syscall::Syscall::A1(f) => f(args.remove(0)),
                    // these are both 0 because args gets mutated, and the second arg is now the first.
                    syscall::Syscall::A2(f) => f(args.remove(0), args.remove(0)),
                };
            }
            None => return Err(format_err!("Couldn't find function for address {:?}", addr)),
        }
    }

    /// Call function an address, allowing it to modify the global state.
    fn call_addr_global(&mut self, addr: Address) -> Result<Literal> {
        let mut ng = self.global.clone();
        let ret = {
            let mut global_ctx = Context::new(self, &mut ng);

            let astfn = self
                .last
                .fr
                .lookup(addr)
                .ok_or_else(|| err_msg("Looking up entry function"))?;

            global_ctx.visit(&astfn.body)
        };
        self.global = ng;
        ret
    }

    /// Import a LiftedAST, executing its entry function.
    pub fn import(&mut self, last: &LiftedAST) -> Result<Literal> {
        let entry = self.last.import(last)?;
        self.call_addr_global(entry)
    }

    pub fn eval(&mut self, a: &AST) -> Result<Literal> {
        let mut ng = self.global.clone();
        let res = self.env_eval(a, &mut ng)?;
        self.global = ng;
        Ok(res)
    }

    pub fn env_eval(&self, a: &AST, env: &mut Env) -> Result<Literal> {
        Context::new(self, env).visit(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast;
    use ast::passes::unbound;
    use data::Literal;
    use parser;
    use parser::Parser;

    fn pi(i: &mut Interpreter, s: &str) -> Result<Literal> {
        let p = Parser::new();
        let lits = &p.parse(s).unwrap()[0];
        let ast = &ast::parse(lits).unwrap();
        i.eval(ast)
    }

    fn last_no_unbound(s: &str) -> LiftedAST {
        let ast = ast::parse_multi(&parser::parse(s).unwrap()).unwrap();
        function_lifter::lift_functions(&ast).unwrap()
    }

    fn pi_last(i: &mut Interpreter, s: &str) -> Result<Literal> {
        let ast = ast::parse_multi(&parser::parse(s).unwrap()).unwrap();
        unbound::pass(&ast, &i.global).unwrap();
        let last = function_lifter::lift_functions(&ast).unwrap();

        i.import(&last)
    }

    #[test]
    fn eval_literal() {
        let mut i = Interpreter::new();
        assert_eq!(pi(&mut i, "4").unwrap(), Literal::Number(4));
    }

    #[test]
    fn eval_boolean() {
        let mut i = Interpreter::new();
        assert_eq!(pi(&mut i, "#t").unwrap(), Literal::Boolean(true));
        assert_eq!(pi(&mut i, "#f").unwrap(), Literal::Boolean(false));
    }

    #[test]
    fn test_if() {
        let mut i = Interpreter::new();

        let p1 = pi(&mut i, "(if #t 1 0)").unwrap();

        assert_eq!(p1, Literal::Number(1));

        let p2 = pi(&mut i, "(if #f 1 0)").unwrap();

        assert_eq!(p2, Literal::Number(0));
    }

    #[test]
    fn test_def() {
        let mut i = Interpreter::new();

        let p1 = pi(&mut i, "(def test 5)").unwrap();

        assert_eq!(p1, Literal::Number(5));
        assert_eq!(pi(&mut i, "test").unwrap(), Literal::Number(5));
    }

    #[test]
    fn test_let() {
        let mut i = Interpreter::new();
        let p1 = pi(&mut i, "(let (test 5) test)").unwrap();
        assert_eq!(p1, Literal::Number(5));
        assert!(pi(&mut i, "test").is_err());
        assert!(i.global.get(&"test".to_string()).is_none());
    }

    #[test]
    fn test_do() {
        let mut i = Interpreter::new();

        let p1 = pi(&mut i, "(do 1 2 3)").unwrap();
        assert_eq!(p1, Literal::Number(3));

        let p2 = pi(&mut i, "(do (def test 4) test)").unwrap();
        assert_eq!(p2, Literal::Number(4));
    }

    #[test]
    fn test_import() {
        let mut i = Interpreter::new();
        assert_eq!(pi_last(&mut i, "1").unwrap(), 1.into());
        assert_eq!(pi_last(&mut i, "(if #f 1 2)").unwrap(), 2.into());
    }

    #[test]
    fn test_lifted_function() {
        let mut i = Interpreter::new();

        assert_eq!(
            pi_last(&mut i, "(def inc (fn (n) (+ n 1))) (inc 1)").unwrap(),
            2.into()
        );
        assert_eq!(pi_last(&mut i, "(inc 5)").unwrap(), 6.into());
        assert_eq!(pi_last(&mut i, "(inc 6)").unwrap(), 7.into());
    }

    #[test]
    fn test_syscalls() {
        let mut i = Interpreter::new();
        assert_eq!(pi_last(&mut i, "(+ 1 2)").unwrap(), 3.into());
        assert_eq!(pi_last(&mut i, "(cons 1 '())").unwrap(), list_lit!(1));
    }

    #[test]
    fn test_with_last() {
        // TODO: add list syscall, but make it throw error telling you to run ast::passes::internal_macro
        let last = last_no_unbound("(def a (fn (x y) (cons x (cons y '()))) )");
        let mut i = Interpreter::with_last(&last).unwrap();

        assert_eq!(pi_last(&mut i, "(a 1 2)").unwrap(), list_lit!(1, 2));
    }
}
