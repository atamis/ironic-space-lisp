//! Tools to manipulate existing bytecodes in functionality maintaining ways
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

/// Extract all literals to literal pool in this bytecode.
///
/// Doesn't modify existing literals in the pool.
pub fn extract_to_pool(code: &mut Bytecode) {
    let mut pool_index = code.pool.len();
    // Mapping between ending literals and eventual index
    let existing_lits: HashMap<Literal, usize> = code
        .pool
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, l)| (l, i))
        .collect();

    let mut new_lits: HashMap<Literal, usize> = HashMap::new();

    for chunk in &mut code.chunks {
        for op in &mut chunk.ops {
            if let Op::Lit(l) = op {
                if !existing_lits.contains_key(l) && !new_lits.contains_key(l) {
                    new_lits.insert(l.clone(), pool_index);
                    pool_index += 1;
                }

                *op = Op::LoadPool(*existing_lits.get(l).or_else(|| new_lits.get(l)).unwrap());
            }
        }
    }

    let mut new_lit_vec = new_lits.into_iter().collect::<Vec<(Literal, usize)>>();
    new_lit_vec.sort_by_key(|(_, i)| *i);

    code.pool
        .append(&mut new_lit_vec.into_iter().map(|(l, _)| l).collect());
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

    #[test]
    fn test_simple_pool_extraction() {
        let v = &mut Bytecode::new(vec![vec![Op::Lit(Literal::from(1))]]);

        extract_to_pool(v);

        assert_eq!(v.pool, vec![Literal::from(1)]);
        assert_eq!(v.chunks[0].ops[0], Op::LoadPool(0))
    }

    #[test]
    fn test_pool_extraction_existing_pool() {
        let v = &mut Bytecode::with_pool(
            vec![vec![Op::LoadPool(0), Op::Lit(Literal::from(1))]],
            vec![Literal::from(2)],
        );

        extract_to_pool(v);

        assert_eq!(v.pool, vec![Literal::from(2), Literal::from(1)]);
        assert_eq!(v.chunks[0].ops[0], Op::LoadPool(0));
        assert_eq!(v.chunks[0].ops[1], Op::LoadPool(1));
    }

    #[test]
    fn test_pool_extraction_dedup() {
        let v = &mut Bytecode::new(vec![
            vec![Op::Lit(Literal::from(1)), Op::Lit(Literal::from(1))],
            vec![Op::Lit(Literal::from(1))],
        ]);

        extract_to_pool(v);

        assert_eq!(v.pool, vec![Literal::from(1)]);
        assert_eq!(v.chunks[0].ops[0], Op::LoadPool(0));
        assert_eq!(v.chunks[0].ops[1], Op::LoadPool(0));
        assert_eq!(v.chunks[1].ops[0], Op::LoadPool(0));
    }

    #[test]
    fn test_pool_extraction_dedup2() {
        let v = &mut Bytecode::with_pool(
            vec![
                vec![Op::Lit(Literal::from(1)), Op::Lit(Literal::from(1))],
                vec![Op::Lit(Literal::from(1))],
                vec![Op::LoadPool(0)],
            ],
            vec![Literal::from(1)],
        );

        extract_to_pool(v);

        assert_eq!(v.pool, vec![Literal::from(1)]);
        assert_eq!(v.chunks[0].ops[0], Op::LoadPool(0));
        assert_eq!(v.chunks[0].ops[1], Op::LoadPool(0));
        assert_eq!(v.chunks[1].ops[0], Op::LoadPool(0));
        assert_eq!(v.chunks[2].ops[0], Op::LoadPool(0));
    }
}
