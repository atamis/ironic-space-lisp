use crate::{data::Literal, vm};
use std::collections::HashMap;
use vm::{bytecode::Bytecode, op::Op};

fn apply_mapping(m: &HashMap<usize, usize>, o: &mut Op) {
    *o = match o.clone() {
        Op::Lit(Literal::Address((c, i))) => Op::Lit(Literal::Address((
            0,
            (m.get(&c).expect("Missing mapping") + i),
        ))),
        Op::Lit(Literal::Closure(arity, (c, i))) => Op::Lit(Literal::Closure(
            arity,
            (0, (m.get(&c).expect("Missing mapping") + i)),
        )),
        x => x,
    }
}

/// Pack a bytecode into a single vector
pub fn pack(code: &Bytecode) -> Vec<Op> {
    // Maps from block index to vector offset
    let mut mapping: HashMap<usize, usize> = HashMap::new();
    let mut v: Vec<Op> = Vec::with_capacity(code.count_ops());

    let mut idx = 0;

    for (i, chunk) in code.chunks.iter().enumerate() {
        mapping.insert(i, idx);

        for op in chunk.ops.iter() {
            idx += 1;
            v.push(op.clone());
        }
    }

    println!("Mapping: {:?}", mapping);

    for op in &mut v {
        apply_mapping(&mapping, op)
    }

    v
}

/// Take a bytecode and produce a new packed bytecode with 1 chunk.
pub fn make_packed(code: &Bytecode) -> Bytecode {
    Bytecode::new(vec![pack(code)])
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let v = pack(&mut Bytecode::new(vec![vec![Op::Return], vec![Op::Pop]]));

        assert_eq!(v, vec![Op::Return, Op::Pop]);
    }

    #[test]
    fn test_first_chunk_addrs() {
        let v = pack(&mut Bytecode::new(vec![vec![
            Op::Lit(Literal::Address((0, 0))),
            Op::Lit(Literal::Address((0, 1))),
            Op::Call,
            Op::Call,
        ]]));

        assert_eq!(
            v,
            vec![
                Op::Lit(Literal::Address((0, 0))),
                Op::Lit(Literal::Address((0, 1))),
                Op::Call,
                Op::Call,
            ]
        );
    }
    #[test]
    fn test_forward_modification() {
        let v = pack(&mut Bytecode::new(vec![
            vec![Op::Lit(Literal::Address((1, 0)))],
            vec![Op::Dup],
        ]));

        assert_eq!(v, vec![Op::Lit(Literal::Address((0, 1))), Op::Dup]);
    }

    #[test]
    fn test_backward_modification() {
        let v = pack(&mut Bytecode::new(vec![
            vec![Op::Dup],
            vec![Op::Pop],
            vec![Op::Lit(Literal::Address((1, 0)))],
        ]));

        assert_eq!(v, vec![Op::Dup, Op::Pop, Op::Lit(Literal::Address((0, 1)))]);
    }
}
