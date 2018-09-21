#[macro_use]
extern crate ironic_space_lisp;

use ironic_space_lisp::ast;
use ironic_space_lisp::ast::passes::internal_macro;
use ironic_space_lisp::ast::passes::unbound;
use ironic_space_lisp::compiler;
use ironic_space_lisp::data::Literal;
use ironic_space_lisp::errors::*;
use ironic_space_lisp::interpreter;
use ironic_space_lisp::parser;
use ironic_space_lisp::self_hosted;
use ironic_space_lisp::vm;

trait Evaler {
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

        self.eval(&ast)
    }
}

struct HostedEvaler(vm::VM);

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

#[test]
fn test_verify() {
    let cases: &[(&str, Option<Literal>)] = &[
        ("1", Some(1.into())),
        ("asdfasdfasdf", None),
        // ("(+ 1 2)", Some(3.into())) // lol
    ];
    let mut evalers: Vec<(&str, Box<Evaler>)> = vec![
        ("vm", Box::new(self_hosted::empty_vm())),
        ("rustint", Box::new(interpreter::Interpreter::new())),
        ("hosted", Box::new(HostedEvaler::new())),
    ];

    for (s, expected) in cases {
        let lit = parser::parse(&s).unwrap();
        for (name, evaler) in evalers.iter_mut() {
            let real = evaler.lit_eval(&lit);
            match (real, expected) {
                (Err(_), None) => {}                                               // good
                (Ok(ref x), Some(ref y)) => assert_eq!(*x, *y, "With {:?}", name), // good
                (Err(ref e), Some(ref y)) => panic!(format!(
                    "With {:?}, Expected: {:?}, got error: {:?}",
                    name, y, e
                )),
                (Ok(x), None) => panic!(format!(
                    "With {:?}, Expected an error, but got: {:?}",
                    name, x
                )),
            }
        }
    }
}
