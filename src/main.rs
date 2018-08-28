extern crate ironic_space_lisp;

use ironic_space_lisp::errors::*;
use ironic_space_lisp::repl;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter_causes() {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = Some(e.backtrace()) {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    println!("Booting repl");

    repl::repl();

    Ok(())
}
