extern crate ironic_space_lisp;

use ironic_space_lisp::vm;
use ironic_space_lisp::vm::Op;
use ironic_space_lisp::vm::data;

fn debug_single_step(vm: &mut vm::VM) {
    vm.single_step();

    println!("{:?}", vm);
}

fn main() {
    let inst = vec![Op::Lit(4), Op::Lit(4), Op::ApplyFunction(Box::new(vm::AdditionFunction))];

    let mut vm = vm::VM::new(inst);

    debug_single_step(&mut vm);
    debug_single_step(&mut vm);
    debug_single_step(&mut vm);

}
