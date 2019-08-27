#[macro_use]
extern crate isl;

extern crate isl_suite;

extern crate serde;
extern crate serde_derive;
extern crate toml;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

use isl::data::Literal;
use isl::interpreter;
use isl::parser;
use isl::self_hosted;

use isl_suite::Evaler;
use isl_suite::HostedEvaler;
use isl_suite::IntHosted;
use isl_suite::SuiteCase;
use isl_suite::SuiteRecord;
use isl_suite::SuiteResult;

fn main() {
    let mut output_buffer = File::create("target/output.toml").unwrap();
    let mut html_buffer = File::create("target/output.html").unwrap();

    let cases: &[(&str, Option<Literal>)] = &[
        ("1", Some(1.into())),
        ("asdfasdfasdf", None),
        ("(+)", None),
        ("(+ 1)", None),
        ("(+ 1 2)", Some(3.into())),
        ("(+ 1 2 3)", None),
        ("(error 'error)", None),
        ("(list 1)", Some(list_lit!(1))),
        ("(list 1 2)", Some(list_lit!(1, 2))),
        ("(list 1 2 3)", Some(list_lit!(1, 2, 3))),
        ("(def x 1) (let [x 2] x)", Some(2.into())),
        ("(def x 1) (def y (fn [] x)) (y)", Some(1.into())),
        (
            "(def x 1) (def y (fn [] x)) (let [x 2] (y))",
            Some(1.into()),
        ),
        (
            // This was n = 100, but got stack overflows from it.
            "(def f (fn (n) (if (= n 0) #t (f (- n 1))))) (f 10)",
            Some(true.into()),
        ),
        ("(def f (fn [x y] x)) (f 1)", None),
        ("(def f (fn [x y] x)) (f 1 2)", Some(1.into())),
        ("(def f (fn [x y] x)) (f 1 2 3)", None),
        ("(let (x 2) (do (def y 1) y))", Some(1.into())),
        ("(def y 3) (let (x 2) (def y 1)) y", Some(3.into())),
    ];
    let mut evalers: Vec<(&str, Box<dyn Evaler>)> = vec![
        ("vm", Box::new(self_hosted::empty_vm())),
        ("rustint", Box::new(interpreter::Interpreter::new())),
        ("hosted", Box::new(HostedEvaler::new())),
        ("inthosted", Box::new(IntHosted::new())),
    ];

    let mut result = SuiteResult { results: vec![] };

    for (s, expected) in cases {
        let lit = parser::parse(&s).unwrap();
        let mut records: HashMap<String, SuiteRecord> = HashMap::new();
        for (name, evaler) in evalers.iter_mut() {
            //println!("{:}, {:?}", s, name);
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
        let case = SuiteCase {
            expr: s.to_string(),
            expected: format!("{:#?}", expected),
            records,
        };

        result.results.push(case);
    }

    println!("Writing toml output");
    output_buffer
        .write_all(toml::to_string_pretty(&result).unwrap().as_bytes())
        .unwrap();

    println!("Writing html output");
    html_buffer
        .write_all(isl_suite::render::render(&result).unwrap().as_bytes())
        .unwrap();
}
