#![recursion_limit = "1024"]

extern crate error_chain;
extern crate ironic_space_lisp;

use ironic_space_lisp::data::make_list;
use ironic_space_lisp::data::Lisp;
use ironic_space_lisp::data::Op;
use ironic_space_lisp::errors::*;
use ironic_space_lisp::vm::*;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let prog = make_list(vec![
        Lisp::Op(Op::Add),
        Lisp::Num(1),
        make_list(vec![Lisp::Op(Op::Add), Lisp::Num(1), Lisp::Num(1)]),
    ]);

    let mut evaler = Evaler::new(prog);
    //let mut evaler = Evaler::new(Lisp::Num(1));
    println!("{:?}", evaler);

    let r = evaler
        .step_until_return()
        .chain_err(|| "Stepping through hardcoded program")?;

    println!("{:?}", r);

    evaler.single_step().chain_err(|| "Single step to fail")?;

    Ok(())
}
