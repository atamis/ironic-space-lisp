extern crate ironic_space_lisp;

use ironic_space_lisp::builtin::ADD;
use ironic_space_lisp::builtin::PRINT;
use ironic_space_lisp::data;
use ironic_space_lisp::errors::*;
use ironic_space_lisp::vm;

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

fn code() -> vm::Bytecode {
    use data::Literal;
    use vm::Bytecode;
    use vm::Op::*;

    let inst0 = vec![
        Lit(Literal::Address((1, 0))),
        Call,
        Dup,
        Lit(Literal::Address(PRINT)),
        Call,
        //Lit(Literal::Address((4, 0))),
        //Call,
        Return,
    ];

    let inst1 = vec![
        Lit(Literal::Number(4)),
        Lit(Literal::Keyword("test".to_string())),
        Store,
        Lit(Literal::Number(4)),
        Lit(Literal::Address((2, 0))),
        Lit(Literal::Address((2, 0))),
        Lit(Literal::Boolean(true)),
        JumpCond,
        //Jump,
        Return,
    ];

    let inst2 = vec![
        Lit(Literal::Keyword("test".to_string())),
        Load,
        Lit(Literal::Address(ADD)),
        Call,
        Lit(Literal::Address((1, 8))),
        Jump,
    ];

    let inst3 = vec![
        Lit(Literal::Number(1)),
        Lit(Literal::Address(ADD)),
        Call,
        Lit(Literal::Address((3, 0))),
        Call,
    ];

    Bytecode::new(vec![inst0, inst1, inst2, inst3])
}

fn run() -> Result<()> {
    let c = code();
    c.dissassemble();
    let mut vm = vm::VM::new(c);

    let r = vm
        .step_until_value(false)
        .context("Execute hardcoded program")?;

    println!("{:?}", vm);
    println!("{:?}", r);

    //println!("{:?}", parser::expr("(test asdf asdf asdf ( asdf  qwerqwer ) )"));
    //println!("{:?}", parser::tokens("(( ((( asdf asdf asdf)))) aa\n asdf    "));
    //println!("{:?}", parser::expr(vec![parser::Token::Open]));

    Ok(())
}
