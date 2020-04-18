use crate::vm;
use vm::{bytecode::Bytecode, op::Op};

/// Pack a bytecode into a single vector
pub fn pack(_code: &mut Bytecode) -> Vec<Op> {
    todo!()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fail() {
        pack(&mut Bytecode::new(vec![vec![]]));
        assert!(false);
    }
}
