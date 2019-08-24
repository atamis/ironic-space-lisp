//! Compile [`AST`](ast::AST)s to [`Bytecode`](vm::bytecode::Bytecode).
use std::rc::Rc;

use crate::ast::passes::function_lifter;
use crate::ast::ASTVisitor;
use crate::ast::Def;
use crate::ast::DefVisitor;
use crate::ast::AST;
use crate::data::Keyword;
use crate::data::Literal;
use crate::errors::*;
use crate::vm::bytecode::Bytecode;
use crate::vm::bytecode::Chunk;
use crate::vm::op::Op;

/// A vector of [`IrOp`]s.
pub type IrChunk = Vec<IrOp>;
/// Alias for an [`IrChunk`] reference.
pub type IrChunkSlice<'a> = &'a [IrOp];

/// Intermediate operation representation.
///
/// As an intermediate representation, it's largely flat, except for [`IrOp::JumpCond`], which
/// represents its potential jump targets as pointers to other IrChunks. Functions
/// are handled by [`function_lifter`] and [`pack_compile_lifted()`] rather
/// represented in IrOp.
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
pub enum IrOp {
    Lit(Literal),
    Return,
    Call,
    Jump,
    JumpCond {
        pred: Rc<IrChunk>,
        then: Rc<IrChunk>,
        els: Rc<IrChunk>,
    },
    Load,
    Store,
    PushEnv,
    PopEnv,
    Dup,
    Pop,
    CallArity(usize),
    Wait,
    Send,
    Fork,
    Pid,
    LoadLocal(usize),
    StoreLocal(usize),
}

/// Empty struct that implements `ASTVisitor<IrChunk>`.
///
/// See `ASTVisitor<IrChunk>` and [`ASTVisitor`] for information.
pub struct Compiler;

impl DefVisitor<IrChunk> for Compiler {
    fn visit_def(&mut self, name: &str, value: &AST) -> Result<IrChunk> {
        let mut body_chunk = self.visit(value)?;

        body_chunk.push(IrOp::Lit(name.into()));
        body_chunk.push(IrOp::Store);

        Ok(body_chunk)
    }
}

impl ASTVisitor<IrChunk> for Compiler {
    fn value_expr(&mut self, l: &Literal) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(l.clone())])
    }

    fn if_expr(&mut self, pred: &Rc<AST>, then: &Rc<AST>, els: &Rc<AST>) -> Result<IrChunk> {
        let pred_chunk = self.visit(pred)?;
        let then_chunk = self.visit(then)?;
        let els_chunk = self.visit(els)?;

        Ok(vec![
            (IrOp::JumpCond {
                pred: Rc::new(pred_chunk),
                then: Rc::new(then_chunk),
                els: Rc::new(els_chunk),
            }),
        ])
    }

    fn def_expr(&mut self, def: &Rc<Def>) -> Result<IrChunk> {
        let mut chunk = self.visit_single_def(def)?;

        chunk.append(&mut self.var_expr(&def.name)?);

        Ok(chunk)
    }

    fn let_expr(&mut self, defs: &[Def], body: &Rc<AST>) -> Result<IrChunk> {
        let mut chunk = vec![IrOp::PushEnv];

        for mut def_chunk in self.visit_multi_def(defs)?.into_iter() {
            chunk.append(&mut def_chunk);
        }

        let mut body_chunk = self.visit(body)?;

        chunk.append(&mut body_chunk);

        chunk.push(IrOp::PopEnv);

        Ok(chunk)
    }

    fn do_expr(&mut self, exprs: &[AST]) -> Result<IrChunk> {
        let mut chunk: IrChunk = vec![];

        let e_chunks = self.multi_visit(&exprs).into_iter().flat_map(|e| e);

        for (idx, mut e_chunk) in e_chunks.enumerate() {
            chunk.append(&mut e_chunk);

            // pop every interstitial value except the last
            if idx != (exprs.len() - 1) {
                chunk.push(IrOp::Pop);
            }
        }

        Ok(chunk)
    }

    fn lambda_expr(&mut self, _args: &[Keyword], _body: &Rc<AST>) -> Result<IrChunk> {
        Err(err_msg(
            "Not implemented: run the function lifter pass first",
        ))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(Literal::Keyword(k.clone())), IrOp::Load])
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &[AST]) -> Result<IrChunk> {
        let mut chunk = vec![];

        for e in args.iter().rev() {
            let mut e_chunk = self.visit(e)?;
            chunk.append(&mut e_chunk);
        }

        let arg_check = |name, arity| {
            if args.len() != arity {
                return Err(err_msg(format!(
                    "{:} takes {:} arguments, given {:}",
                    name,
                    arity,
                    args.len()
                )));
            } else {
                Ok(())
            }
        };

        // Ideally this would be handled by a combined else
        // clause, ie, the match expression would match over
        // the struct rather than the string, but that doesn't
        // work, so we combine the else clauses of the match and the
        // if let with this bool.
        let mut normal_call = false;

        if let AST::Var(s) = &**f {
            match s.as_ref() {
                "fork" => {
                    arg_check("fork", 0)?;
                    chunk.push(IrOp::Fork);
                }
                "wait" => {
                    arg_check("fork", 0)?;
                    chunk.push(IrOp::Wait);
                }
                "send" => {
                    arg_check("send", 2)?;
                    chunk.push(IrOp::Send);
                }
                "pid" => {
                    arg_check("pid", 0)?;
                    chunk.push(IrOp::Pid);
                }

                _ => normal_call = true,
            };
        } else {
            normal_call = true;
        }

        if normal_call {
            let mut f_chunk = self.visit(f)?;
            chunk.append(&mut f_chunk);

            chunk.push(IrOp::CallArity(args.len()));
        }

        Ok(chunk)
    }
}

/// Compiles a raw [ `AST` ] into an [ `IrChunk` ]. See [ `Compiler` ] for implementation.
pub fn compile(a: &AST) -> Result<IrChunk> {
    let mut c = Compiler {};
    c.visit(a)
}

// Allocate an empty chunk and return its idx.
// This could be much more sophisticated, but isn't.
fn alloc_chunk(code: &mut Bytecode) -> usize {
    let idx = code.chunks.len();
    code.chunks.push(Chunk { ops: vec![] });
    idx
}

/// Compile and pack a [`LiftedAST`](function_lifter::LiftedAST) into a new bytecode.
pub fn pack_compile_lifted(last: &function_lifter::LiftedAST) -> Result<Bytecode> {
    let mut code = Bytecode::new(vec![]);

    // allocate chunks first
    for (id, _) in last.fr.functions.iter().enumerate() {
        let chunk = alloc_chunk(&mut code);
        if id != chunk {
            panic!("id chunk missalignment");
        }
    }

    // load functions into the chunks
    for (id, function) in last.fr.functions.iter().enumerate() {
        let chunk = id;
        let is_entry = id == last.entry;

        let mut ir = compile(&function.body)?;

        if !is_entry {
            ir.push(IrOp::PopEnv);
        }
        ir.push(IrOp::Return);

        let mut arg_ir: IrChunk = function
            .args
            .iter()
            .map(|k| vec![IrOp::Lit(Literal::Keyword(k.clone())), IrOp::Store])
            .flat_map(|x| x)
            .collect();

        if !is_entry {
            arg_ir.insert(0, IrOp::PushEnv);
        }

        arg_ir.append(&mut ir);

        if !is_entry {
            tail_call_optimization(&mut arg_ir);
        }

        pack(&arg_ir, &mut code, chunk, 0)?;
    }

    // function 0 is a dummy function in FunctionRegistry, so stick the root there.
    //code.chunks[0].ops.clear();

    //let mut root_ir = compile(&last.root)?;
    //root_ir.push(IrOp::Return);

    //pack(&root_ir, &mut code, 0, 0)?;

    Ok(code)
}

// This doesn't really work.
fn tail_call_optimization(chunk: &mut IrChunk) {
    let len = chunk.len();
    if len >= 3
        && chunk[len - 3] == IrOp::Call
        && chunk[len - 2] == IrOp::PopEnv
        && chunk[len - 1] == IrOp::Return
    {
        chunk[len - 3] = IrOp::PopEnv;
        chunk[len - 2] = IrOp::Jump;
    }
}

/// Pack an [ `IrChunk` ] into a new [ `Bytecode` ] and return it.
pub fn pack_start(ir: IrChunkSlice) -> Result<Bytecode> {
    let mut code = Bytecode::new(vec![vec![]]);

    let chunk_idx = alloc_chunk(&mut code);

    pack(ir, &mut code, chunk_idx, 0)?;

    code.chunks[0].ops.append(&mut vec![
        Op::Lit(Literal::Address((chunk_idx, 0))),
        Op::Call,
        Op::Return,
    ]);

    code.chunks[chunk_idx].ops.push(Op::Return);

    Ok(code)
}

/// Pack an [ `IrChunk` ] into bytecode at a particular chunk and op index. Returns ending op index.
pub fn pack(
    ir: IrChunkSlice,
    code: &mut Bytecode,
    chunk_idx: usize,
    op_idx: usize,
) -> Result<usize> {
    let mut op_idx = op_idx;

    for ir_op in ir.iter() {
        let new_op = match ir_op {
            IrOp::Lit(l) => Op::Lit(l.clone()),
            IrOp::Return => Op::Return,
            IrOp::Call => Op::Call,
            IrOp::Load => Op::Load,
            IrOp::Store => Op::Store,
            IrOp::PushEnv => Op::PushEnv,
            IrOp::PopEnv => Op::PopEnv,
            IrOp::Dup => Op::Dup,
            IrOp::Pop => Op::Pop,
            IrOp::Jump => Op::Jump,
            IrOp::JumpCond { pred, then, els } => {
                let els_idx = alloc_chunk(code);
                pack(els, code, els_idx, 0)?;

                let then_idx = alloc_chunk(code);
                pack(then, code, then_idx, 0)?;

                code.chunks[chunk_idx]
                    .ops
                    .push(Op::Lit(Literal::Address((els_idx, 0))));
                op_idx += 1;
                code.chunks[chunk_idx]
                    .ops
                    .push(Op::Lit(Literal::Address((then_idx, 0))));
                op_idx += 1;

                op_idx = pack(pred, code, chunk_idx, op_idx)?;

                let res_idx = op_idx + 1;
                let mut ret_code = vec![Op::Lit(Literal::Address((chunk_idx, res_idx))), Op::Jump];

                code.chunks[els_idx].ops.append(&mut ret_code.clone());
                code.chunks[then_idx].ops.append(&mut ret_code);

                Op::JumpCond
            }
            IrOp::CallArity(a) => Op::CallArity(*a),
            IrOp::Wait => Op::Wait,
            IrOp::Send => Op::Send,
            IrOp::Fork => Op::Fork,
            IrOp::Pid => Op::Pid,
            IrOp::LoadLocal(idx) => Op::LoadLocal(*idx),
            IrOp::StoreLocal(idx) => Op::StoreLocal(*idx),
            //_ => { return Err(err_msg("not implemented"))},
        };

        code.chunks[chunk_idx].ops.push(new_op);
        op_idx += 1;
    }

    Ok(op_idx)
}

// value -> Op
// var -> Vec<Op>
// application -> Vec<Op.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::str_to_ast;
    use crate::vm::VM;
    use test::Bencher;

    fn run(s: &'static str) -> Result<Literal> {
        let ast = str_to_ast(s)?;

        let ir = compile(&ast)?;

        let code = pack_start(&ir)?;

        let mut vm = VM::new(code);

        vm.step_until_cost(10000).map(Option::unwrap)
    }

    #[test]
    fn test_values() {
        assert_eq!(run("4").unwrap(), Literal::Number(4));
        assert_eq!(run("5").unwrap(), Literal::Number(5));
    }

    #[test]
    fn test_def() {
        assert_eq!(run("(def test 5) test").unwrap(), Literal::Number(5));
        assert_eq!(
            run("(def test 5) test (def asdf 0) asdf").unwrap(),
            Literal::Number(0)
        );
    }

    #[test]
    fn test_let() {
        assert_eq!(run("(let (x 1 y 2) x)").unwrap(), Literal::Number(1));
        assert_eq!(run("(let (x 1 y 2) y)").unwrap(), Literal::Number(2));
    }

    fn lifted_compile(s: &'static str) -> Bytecode {
        let ast = str_to_ast(s).unwrap();
        let last = function_lifter::lift_functions(&ast).unwrap();

        pack_compile_lifted(&last).unwrap()
    }

    #[test]
    fn test_pack_compile_lifted() {
        let code = lifted_compile("(def x (lambda () 5)) (x)");

        let mut vm = VM::new(code);

        vm.code.dissassemble();

        assert_eq!(vm.step_until_cost(10000).unwrap(), Some(Literal::Number(5)));
    }

    #[test]
    fn test_pack_compile_lifted_arguments() {
        let code = lifted_compile("(def x (lambda (y z) z)) (x 5 6)");

        let mut vm = VM::new(code);

        assert_eq!(vm.step_until_cost(10000).unwrap(), Some(Literal::Number(6)));
    }

    #[test]
    fn test_pack_compile_lifted_env() {
        let code = lifted_compile("(def x (lambda (y) y)) (let (y 4) (do (x 5) y))");

        code.dissassemble();

        let mut vm = VM::new(code);

        assert_eq!(vm.step_until_cost(10000).unwrap(), Some(Literal::Number(4)));
    }

    #[test]
    fn test_do_pops() {
        let code = lifted_compile("(do 0 1 2 3 4)");

        code.dissassemble();

        let mut vm = VM::new(code);

        assert_eq!(vm.step_until_cost(10000).unwrap(), Some(Literal::Number(4)));

        assert!(vm.stack.is_empty());
    }

    #[test]
    fn test_infinite_recursion() {
        let code = lifted_compile("(def x (lambda () (x))) (x)");

        code.dissassemble();

        let mut vm = VM::new(code);

        assert_eq!(vm.step_until_cost(10000).unwrap(), None);

        println!("{:?}", vm);
    }

    #[test]
    fn test_arity_checking() {
        let code = lifted_compile("(def test (lambda (x y) (do x y))) (test 1)");

        code.dissassemble();

        let mut vm = VM::new(code);

        let res = vm.step_until_cost(10000);

        println!("{:?}", res);

        assert!(res.is_err())
    }

    #[bench]
    fn bench_toolchain(b: &mut Bencher) {
        use test;
        b.iter(|| {
            let ast = str_to_ast("(def add1 (lambda (x) (let (y 1) (+ 1 1)))) (add1 5)").unwrap();

            let last = function_lifter::lift_functions(&ast).unwrap();

            test::black_box(pack_compile_lifted(&last).unwrap());
        })
    }
}
