#[macro_use]
extern crate isl;

use isl::ast;
use isl::ast::passes::function_lifter;
use isl::ast::passes::internal_macro;
use isl::ast::passes::unbound;
use isl::compiler;
use isl::data::Literal;
use isl::errors::*;
use isl::interpreter;
use isl::parser;
use isl::self_hosted;
use isl::vm;

pub trait Evaler {
    fn lit_eval(&mut self, lit: &[Literal]) -> Result<Literal>;
}

impl Evaler for vm::VM {
    fn lit_eval(&mut self, lit: &[Literal]) -> Result<Literal> {
        let last = self_hosted::ast(&lit, self.environment.peek().unwrap())?;

        let code = compiler::pack_compile_lifted(&last).unwrap();

        self.import_jump(&code);

        self.step_until_value(false)
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

        let last = self_hosted::ast(&lits, vm.environment.peek().unwrap()).unwrap();

        let code = compiler::pack_compile_lifted(&last).unwrap();

        vm.import_jump(&code);

        vm.step_until_value(false).unwrap();

        HostedEvaler(vm)
    }
}

impl Evaler for HostedEvaler {
    fn lit_eval(&mut self, lits: &[Literal]) -> Result<Literal> {
        let vm = &mut self.0;

        let mut lits = lits.to_vec();
        lits.insert(0, "do".into());

        // (ret-v (eval (quote (do *lits)) (quote ())))
        let caller = list_lit!(
            "ret-v",
            list_lit!(
                "eval",
                list_lit!("quote", lits),
                list_lit!("quote", list_lit!())
            )
        );

        println!("caller: {:?}", caller);

        let last = self_hosted::ast(&[caller], vm.environment.peek()?)?;

        vm.import_jump(&compiler::pack_compile_lifted(&last)?);

        vm.step_until_value(false)
    }
}
