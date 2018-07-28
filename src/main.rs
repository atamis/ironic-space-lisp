extern crate ironic_space_lisp;

use ironic_space_lisp::vm::*;
use ironic_space_lisp::data::Lisp;
use ironic_space_lisp::data::Op;
use ironic_space_lisp::data::make_list;

fn main() {
    let prog = make_list(vec![
        Lisp::Op(Op::Add),
        Lisp::Num(1),
        make_list(vec![Lisp::Op(Op::Add), Lisp::Num(1), Lisp::Num(1)]),
    ]);

    let mut evaler = Evaler::new(prog);
    //let mut evaler = Evaler::new(Lisp::Num(1));
    println!("{:?}", evaler);

    println!("{:?}", evaler.step_until_return());
}
