//! Bytecode containers and dissassembly.

use data::Address;
use data::Literal;
use errors::*;
use std::fmt;
use vm::op::Op;

/// Holds `Chunk`s of bytecode. See `Bytecode::addr` for its primary use.
#[derive(Clone, PartialEq)]
pub struct Bytecode {
    /// Vec of chunks.
    pub chunks: Vec<Chunk>,
}

/// A `Vec` of operations
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// Vec of operations.
    pub ops: Vec<Op>,
}

impl Chunk {
    /// Pretty prints the operations in the chunk to standard out.
    ///
    /// Separates the fields of the dissassembly with tabs, prints address,
    /// operation name, and any extra information (like from [`Op::Lit`] and [`Op::CallArity`]).
    /// Uses [`Op::dissassemble`], is used by [`Bytecode::dissassemble`].
    pub fn dissassemble(&self, chunk_idx: usize) {
        for (op_idx, op) in self.ops.iter().enumerate() {
            let a = (chunk_idx, op_idx);

            print!("\t{:?}\t{:}", a, op.dissassemble());

            if let Op::Lit(l) = op {
                print!("\t{:?}", l);
            }
            if let Op::CallArity(a) = op {
                print!("\t{:}", a);
            }

            println!()
        }
    }
}

impl fmt::Debug for Bytecode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bytecode {{compiled code}}")
    }
}

impl Bytecode {
    /// Create a new bytecode from a double vector of operations.
    ///
    /// The 2nd level vectors are converted to chunks.
    pub fn new(v: Vec<Vec<Op>>) -> Bytecode {
        Bytecode {
            chunks: v.into_iter().map(|c| Chunk { ops: c }).collect(),
        }
    }

    /// Indexes into the chunks to find the indicated operation.
    pub fn addr(&self, a: Address) -> Result<Op> {
        let chunk = self
            .chunks
            .get(a.0)
            .ok_or_else(|| format_err!("Invalid chunk address: {:?}", a))?;
        let op = chunk
            .ops
            .get(a.1)
            .ok_or_else(|| format_err!("Invalid operation address: {:?}", a))?;
        Ok(op.clone())
    }

    /// Prints a plain text disassembly of all the chunks to STDOUT.
    pub fn dissassemble(&self) {
        for (chunk_idx, chunk) in self.chunks.iter().enumerate() {
            println!("################ CHUNK #{:?} ################", chunk_idx);
            chunk.dissassemble(chunk_idx);
        }
    }

    /// Import chunks from another [`Bytecode`], returning the address of the first imported chunk.
    ///
    /// This clones the bytecode, and rewrites the addresses so that they point to the same
    /// chunks.
    pub fn import(&mut self, code: &Bytecode) -> Address {
        let new_chunk_idx = self.chunks.len();

        let mut new_chunks: Vec<Chunk> = code
            .chunks
            .iter()
            .cloned()
            .map(|chunk| Chunk {
                ops: chunk
                    .ops
                    .iter()
                    .map(|op| match op {
                        Op::Lit(Literal::Address((a1, a2))) => {
                            Op::Lit(Literal::Address((a1 + new_chunk_idx, *a2)))
                        }
                        Op::Lit(Literal::Closure(arity, (a1, a2))) => {
                            Op::Lit(Literal::Closure(*arity, ((a1 + new_chunk_idx), *a2)))
                        }
                        x => x.clone(),
                    })
                    .collect(),
            })
            .collect();

        self.chunks.append(&mut new_chunks);

        (new_chunk_idx, 0)
    }
}
