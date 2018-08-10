extern crate ironic_space_lisp;

use ironic_space_lisp::data;
use ironic_space_lisp::errors::*;
use ironic_space_lisp::vm;
use ironic_space_lisp::builtin::ADD;

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

fn code() -> vm::Bytecode {
    use data::Literal;
    use vm::Bytecode;
    use vm::Chunk;
    use vm::Op::*;

    let inst0 = vec![
        Lit(Literal::Number(4)),
        Lit(Literal::Number(4)),
        Lit(Literal::Number(0)),
        Lit(Literal::Address((1, 0))),
        IfZ,
        //Jump,
        Return,
    ];

    let inst1 = vec![Lit(Literal::Address(ADD)), Call, Return];

    Bytecode {
        chunks: vec![Chunk { ops: inst0 }, Chunk { ops: inst1 }],
    }
}

fn run() -> Result<()> {
    let mut vm = vm::VM::new(code());

    let r = vm
        .step_until_value(true)
        .chain_err(|| "Execute hardcoded program")?;

    println!("{:?}", vm);
    println!("{:?}", r);

    Ok(())
}
