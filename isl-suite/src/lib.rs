#[macro_use]
extern crate isl;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate toml;

use std::collections::HashMap;

use isl::ast;
use isl::ast::passes::function_lifter;
use isl::ast::passes::internal_macro;
use isl::ast::passes::local;
use isl::ast::passes::unbound;
use isl::compiler;
use isl::data::Literal;
use isl::env;
use isl::errors::*;
use isl::interpreter;
use isl::parser;
use isl::self_hosted;
use isl::vm;

pub mod render;

#[derive(Serialize, Debug)]
pub struct SuiteRecord {
    pub ok: bool,
    pub actual: String,
}

#[derive(Serialize, Debug)]
pub struct SuiteCase {
    pub expr: String,
    pub expected: String,
    pub records: HashMap<String, SuiteRecord>,
}

#[derive(Serialize, Debug)]
pub struct SuiteResult {
    pub results: Vec<SuiteCase>,
}

pub trait Evaler {
    fn lit_eval(&mut self, lit: &[Literal]) -> Result<Literal>;
}

impl Evaler for vm::VM {
    fn lit_eval(&mut self, lit: &[Literal]) -> Result<Literal> {
        let last = ast::ast(&lit, self.environment.peek().unwrap())?;

        let code = compiler::compile(&last).unwrap();

        self.import_jump(&code);

        self.step_until_value()
    }
}

impl Evaler for interpreter::Interpreter {
    fn lit_eval(&mut self, lits: &[Literal]) -> Result<Literal> {
        let ast = ast::parse_multi(&lits)?;
        let ast = internal_macro::pass(&ast)?;

        unbound::pass(&ast, &self.global)?;

        let last = function_lifter::lift_functions(&ast)?;

        self.import(&last)
    }
}

pub struct HostedEvaler(vm::VM);

impl Default for HostedEvaler {
    fn default() -> Self {
        Self::new()
    }
}

impl HostedEvaler {
    pub fn new() -> HostedEvaler {
        let mut vm = self_hosted::empty_vm();

        let s = self_hosted::read_lisp().unwrap();

        let lits = parser::parse(&s).unwrap();

        let last = ast::ast(&lits, vm.environment.peek().unwrap()).unwrap();

        let code = compiler::compile(&last).unwrap();

        vm.import_jump(&code);

        vm.step_until_value().unwrap();

        HostedEvaler(vm)
    }
}

fn hosted_launcher(lits: &[Literal], _env: &env::Env) -> Literal {
    let mut lits = lits.to_vec();
    lits.insert(0, "do".into());

    // (ret-v (eval (quote (do *lits)) (quote ())))
    list_lit!(
        "ret-v",
        list_lit!(
            "eval",
            list_lit!("quote", lits),
            list_lit!("quote", list_lit!())
        )
    )
}

fn hosted_launcher_llast(lits: &[Literal], env: &env::Env) -> Result<local::LocalLiftedAST> {
    let launcher_lits = hosted_launcher(lits, env);

    let llast = ast::ast(&[launcher_lits], &env)?;

    Ok(llast)
}

fn hosted_launcher_last(lits: &[Literal], env: &env::Env) -> Result<function_lifter::LiftedAST> {
    let launcher_lits = hosted_launcher(lits, env);

    let ast = ast::parse_multi(&[launcher_lits]).unwrap();

    let ast = internal_macro::pass(&ast).unwrap();

    unbound::pass(&ast, env).unwrap();

    let last = function_lifter::lift_functions(&ast).unwrap();

    Ok(last)
}

impl Evaler for HostedEvaler {
    fn lit_eval(&mut self, lits: &[Literal]) -> Result<Literal> {
        let vm = &mut self.0;

        let llast = hosted_launcher_llast(lits, vm.environment.peek()?)?;

        vm.import_jump(&compiler::compile(&llast)?);

        vm.step_until_value()
    }
}

#[derive(Default)]
pub struct IntHosted {
    terp: interpreter::Interpreter,
}

impl IntHosted {
    #[allow(unused_must_use)]
    pub fn new() -> IntHosted {
        let mut terp = interpreter::Interpreter::new();

        let s = self_hosted::read_lisp().unwrap();

        let lits = parser::parse(&s).unwrap();

        let ast = ast::parse_multi(&lits).unwrap();

        let ast = internal_macro::pass(&ast).unwrap();

        unbound::pass(&ast, &terp.global).unwrap();

        let last = function_lifter::lift_functions(&ast).unwrap();

        // This returns an error, but it still works.
        terp.import(&last);

        IntHosted { terp }
    }
}

impl Evaler for IntHosted {
    fn lit_eval(&mut self, lits: &[Literal]) -> Result<Literal> {
        match self
            .terp
            .import(&hosted_launcher_last(lits, &self.terp.global)?)
        {
            Ok(r) => Ok(r),
            Err(e) => Err(err_msg(format!("Elided error: {}", e))),
        }
    }
}
