#[macro_use]
extern crate isl;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

use isl::ast;
use isl::ast::passes::internal_macro;
use isl::ast::passes::unbound;
use isl::compiler;
use isl::data::Literal;
use isl::errors::*;
use isl::interpreter;
use isl::parser;
use isl::self_hosted;
use isl::vm;

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

#[derive(Serialize, Debug)]
struct SuiteRecord {
    ok: bool,
    actual: String,
}

#[derive(Serialize, Debug)]
struct SuiteCase {
    expr: String,
    expected: String,
    records: HashMap<String, SuiteRecord>,
}

#[derive(Serialize, Debug)]
struct SuiteResult {
    results: Vec<SuiteCase>,
}

fn main() {
    let mut output_buffer = File::create("target/output.toml").unwrap();

    let cases: &[(&str, Option<Literal>)] = &[
        ("1", Some(1.into())),
        ("asdfasdfasdf", None),
        ("(+ 1 2)", Some(3.into())), // lol
        ("(list 1)", Some(list_lit!(1))),
        ("(list 1 2)", Some(list_lit!(1, 2))),
        ("(list 1 2 3)", Some(list_lit!(1, 2, 3))),
    ];
    let mut evalers: Vec<(&str, Box<Evaler>)> = vec![
        ("vm", Box::new(self_hosted::empty_vm())),
        ("rustint", Box::new(interpreter::Interpreter::new())),
        ("hosted", Box::new(HostedEvaler::new())),
    ];

    let mut result = SuiteResult { results: vec![] };

    for (s, expected) in cases {
        let lit = parser::parse(&s).unwrap();
        let mut records: HashMap<String, SuiteRecord> = HashMap::new();
        for (name, evaler) in evalers.iter_mut() {
            let real = evaler.lit_eval(&lit);

            let ok = match (&real, expected) {
                (Err(_), None) => true,
                (Ok(ref x), Some(ref y)) if x == y => true,
                (Ok(ref _x), Some(ref _y)) => false, // else above
                (Err(_), Some(_)) => false,
                (Ok(_), None) => false,
            };

            let res = SuiteRecord {
                actual: format!("{:#?}", real),
                ok,
            };

            records.insert(name.to_string(), res);
        }
        result.results.push(SuiteCase {
            expr: s.to_string(),
            expected: format!("{:#?}", expected),
            records,
        });
    }

    output_buffer
        .write(toml::to_string_pretty(&result).unwrap().as_bytes())
        .unwrap();
}
