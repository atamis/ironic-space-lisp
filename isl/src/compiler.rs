//! Compile [`AST`](ast::AST)s to [`Bytecode`](vm::bytecode::Bytecode).
use std::rc::Rc;

use crate::ast::passes::local;
use crate::ast::passes::local::visitors;
use crate::ast::passes::local::visitors::GlobalDefVisitor;
use crate::ast::passes::local::visitors::LLASTVisitor;
use crate::ast::passes::local::visitors::LocalASTVisitor;
use crate::ast::passes::local::visitors::LocalDefVisitor;
use crate::ast::passes::local::GlobalDef;
use crate::ast::passes::local::LocalAST;
use crate::data::Literal;
use crate::data::Symbol;
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
/// are handled by [`function_lifter`] and [`compile()`] rather
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
    Terminate,
}

/// Empty struct that implements `ASTVisitor<IrChunk>`.
///
/// See `ASTVisitor<IrChunk>` and [`ASTVisitor`] for information.
pub struct Compiler;

impl visitors::GlobalDefVisitor<IrChunk> for Compiler {
    fn visit_globaldef(&mut self, name: &Symbol, value: &LocalAST) -> Result<IrChunk> {
        let mut body_chunk = self.visit(value)?;

        body_chunk.push(IrOp::Lit(name.clone().into()));
        body_chunk.push(IrOp::Store);

        Ok(body_chunk)
    }
}

impl visitors::LocalDefVisitor<IrChunk> for Compiler {
    fn visit_localdef(&mut self, index: usize, value: &LocalAST) -> Result<IrChunk> {
        let mut body_chunk = self.visit(value)?;

        body_chunk.push(IrOp::StoreLocal(index));

        Ok(body_chunk)
    }
}

impl visitors::LocalASTVisitor<IrChunk> for Compiler {
    fn value_expr(&mut self, l: &Literal) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(l.clone())])
    }

    fn if_expr(
        &mut self,
        pred: &Rc<LocalAST>,
        then: &Rc<LocalAST>,
        els: &Rc<LocalAST>,
    ) -> Result<IrChunk> {
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

    fn def_expr(&mut self, def: &Rc<GlobalDef>) -> Result<IrChunk> {
        let mut chunk = self.visit_single_globaldef(def)?;

        chunk.append(&mut self.globalvar_expr(&def.name)?);

        Ok(chunk)
    }

    fn let_expr(&mut self, defs: &[local::LocalDef], body: &Rc<LocalAST>) -> Result<IrChunk> {
        let mut chunk = vec![IrOp::PushEnv];

        for mut def_chunk in self.visit_multi_localdef(defs)?.into_iter() {
            chunk.append(&mut def_chunk);
        }

        let mut body_chunk = self.visit(body)?;

        chunk.append(&mut body_chunk);

        chunk.push(IrOp::PopEnv);

        Ok(chunk)
    }

    fn do_expr(&mut self, exprs: &[LocalAST]) -> Result<IrChunk> {
        let mut chunk: IrChunk = vec![];

        let e_chunks = self
            .multi_visit(&exprs)
            .context("Visiting do expr bodies")?
            .into_iter();

        for (idx, mut e_chunk) in e_chunks.enumerate() {
            chunk.append(&mut e_chunk);

            // pop every interstitial value except the last
            if idx != (exprs.len() - 1) {
                chunk.push(IrOp::Pop);
            }
        }

        Ok(chunk)
    }

    fn localdef_expr(&mut self, def: &Rc<local::LocalDef>) -> Result<IrChunk> {
        let mut chunk = self.visit_single_localdef(def)?;

        chunk.append(
            &mut self
                .localvar_expr(def.name)
                .context("While visiting the value return part")?,
        );

        Ok(chunk)
    }

    fn globalvar_expr(&mut self, name: &Symbol) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(Literal::Symbol(name.clone())), IrOp::Load])
    }

    fn localvar_expr(&mut self, index: usize) -> Result<IrChunk> {
        Ok(vec![IrOp::LoadLocal(index)])
    }

    fn application_expr(&mut self, f: &Rc<LocalAST>, args: &[LocalAST]) -> Result<IrChunk> {
        let mut chunk = vec![];

        for e in args.iter().rev() {
            let mut e_chunk = self.visit(e)?;
            chunk.append(&mut e_chunk);
        }

        let arg_check = |name, arity| {
            if args.len() != arity {
                Err(err_msg(format!(
                    "{:} takes {:} arguments, given {:}",
                    name,
                    arity,
                    args.len()
                )))
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

        if let LocalAST::GlobalVar(s) = &**f {
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
                "terminate" => {
                    arg_check("terminate", 1)?;
                    chunk.push(IrOp::Terminate);
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

impl visitors::LLASTVisitor<IrChunk> for Compiler {
    fn visit_local_function(
        &mut self,
        args: &[Symbol],
        body: &Rc<LocalAST>,
        entry: bool,
    ) -> Result<IrChunk> {
        let mut ir = self.visit(body)?;

        if !entry {
            ir.push(IrOp::PopEnv);
        }

        ir.push(IrOp::Return);

        let mut arg_ir: IrChunk = args
            .iter()
            .enumerate()
            .map(|(i, _)| IrOp::StoreLocal(i))
            .collect();

        if !entry {
            arg_ir.insert(0, IrOp::PushEnv);
        }

        arg_ir.append(&mut ir);

        if !entry {
            tail_call_optimization(&mut arg_ir);
        }

        Ok(arg_ir)
    }
}

// Allocate an empty chunk and return its idx.
// This could be much more sophisticated, but isn't.
fn alloc_chunk(code: &mut Bytecode) -> usize {
    let idx = code.chunks.len();
    code.chunks.push(Chunk { ops: vec![] });
    idx
}

/// Compile and pack a [`LiftedAST`](function_lifter::LiftedAST) into a new bytecode.
pub fn compile(llast: &local::LocalLiftedAST) -> Result<Bytecode> {
    let mut code = Bytecode::new(vec![]);

    // allocate chunks first
    // The previous compiler phases assume that then nth function is in the nth chunk
    // This is how the packing works later in the function, and how the previous passes
    // lift functions and replace them with addresses or closures.
    for (id, _) in llast.functions.iter().enumerate() {
        let chunk = alloc_chunk(&mut code);
        if id != chunk {
            panic!("id chunk missalignment");
        }
    }

    let mut c = Compiler {};

    for (id, chunk) in c.llast_visit(llast)?.into_iter().enumerate() {
        pack(&chunk, &mut code, id, 0)?;
    }

    Ok(code)
}

fn tail_call_optimization(chunk: &mut IrChunk) {
    use IrOp::*;
    let len = chunk.len();

    if len >= 3 {
        let tc = match (&chunk[len - 3], &chunk[len - 2], &chunk[len - 1]) {
            (Call, PopEnv, Return) => true,
            (CallArity(_), PopEnv, Return) => true,
            _ => false,
        };

        if tc {
            chunk[len - 3] = IrOp::PopEnv;
            chunk[len - 2] = IrOp::Jump;
        }
    }
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
            IrOp::Terminate => Op::Terminate,
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
    use crate::ast;
    use crate::ast::passes::function_lifter;
    use crate::parser;
    use crate::str_to_ast;
    use crate::vm::bytecode;
    use crate::vm::VM;
    use test::Bencher;

    fn run(s: &'static str) -> Result<Literal> {
        let lits = parser::parse(s)?;

        let mut vm = VM::new(bytecode::Bytecode::new(vec![vec![]]));

        let ast = ast::ast(&lits, vm.environment.peek()?)?;

        let code = compile(&ast)?;

        vm.import_jump(&code);

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
        let llast = local::pass(&last).unwrap();

        compile(&llast).unwrap()
    }

    #[test]
    fn test_compile() {
        let code = lifted_compile("(def x (lambda () 5)) (x)");

        let mut vm = VM::new(code);

        vm.code.dissassemble();

        assert_eq!(vm.step_until_cost(10000).unwrap(), Some(Literal::Number(5)));
    }

    #[test]
    fn test_compile_arguments() {
        let code = lifted_compile("(def x (lambda (y z) z)) (x 5 6)");

        code.dissassemble();

        let mut vm = VM::new(code);

        assert_eq!(vm.step_until_cost(10000).unwrap(), Some(Literal::Number(6)));
    }

    #[test]
    fn test_compile_env() {
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

    #[test]
    fn test_localdefs() {
        let code = lifted_compile("(let (x 2) (do (def y 1) y))");

        code.dissassemble();

        let mut vm = VM::new(code);

        let res = vm.step_until_cost(10000);

        println!("{:?}", res);

        assert_eq!(res.unwrap().unwrap(), 1.into())
    }

    #[test]
    fn test_localdefs2() {
        let code = lifted_compile("(def y 3) (let (x 2) (def y 1)) y");

        code.dissassemble();

        let mut vm = VM::new(code);

        let res = vm.step_until_cost(10000);

        println!("{:?}", res);

        assert_eq!(res.unwrap().unwrap(), 3.into())
    }

    #[test]
    fn test_async_ops_execution() {
        use crate::exec;

        let code = lifted_compile("(let (me (pid)) (if (fork) (send me 'hello) (wait)))");

        code.dissassemble();

        let mut exec = exec::Exec::new();

        let vm = VM::new(Bytecode::new(vec![vec![]]));

        let (_vm, res) = exec.sched(vm, &code);

        assert_eq!(res.unwrap(), "hello".into())
    }

    #[test]
    fn test_async_ops() {
        let code = lifted_compile("(fork)");

        assert_eq!(code.addr((0, 0)).unwrap(), Op::Fork);
        assert_eq!(code.addr((0, 1)).unwrap(), Op::Return);

        let code = lifted_compile("(wait)");

        code.dissassemble();

        assert_eq!(code.addr((0, 0)).unwrap(), Op::Wait);
        assert_eq!(code.addr((0, 1)).unwrap(), Op::Return);

        let code = lifted_compile("(pid)");

        code.dissassemble();

        assert_eq!(code.addr((0, 0)).unwrap(), Op::Pid);
        assert_eq!(code.addr((0, 1)).unwrap(), Op::Return);

        let code = lifted_compile("(send (pid) 'test)");

        code.dissassemble();

        assert_eq!(code.addr((0, 1)).unwrap(), Op::Pid);
        assert_eq!(code.addr((0, 2)).unwrap(), Op::Send);
        assert_eq!(code.addr((0, 3)).unwrap(), Op::Return);
    }
    #[test]
    fn test_async_ops_arity() {
        assert!(run("(fork 1)").is_err());
        assert!(run("(wait 1)").is_err());
        assert!(run("(pid 1)").is_err());
        assert!(run("(send 1)").is_err());
        assert!(run("(send)").is_err());
    }

    #[test]
    fn test_terminate() {
        let code =
            lifted_compile("(def s (lambda (n) (if (= n 0) (terminate 'ok) (s (- n 1))))) (s 10)");

        code.dissassemble();

        let mut vm = VM::new(code);

        let val = vm.step_until_value().unwrap();

        assert_eq!(val, "ok".into());
        assert!(vm.stack.is_empty());
        assert!(vm.frames.is_empty());
    }

    #[bench]
    fn bench_toolchain(b: &mut Bencher) {
        use test;
        b.iter(|| {
            let ast = str_to_ast("(def add1 (lambda (x) (let (y 1) (+ 1 1)))) (add1 5)").unwrap();

            let last = function_lifter::lift_functions(&ast).unwrap();

            let llast = local::pass(&last).unwrap();

            test::black_box(compile(&llast).unwrap());
        })
    }
}
