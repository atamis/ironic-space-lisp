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
    LoadLocal(usize),
    StoreLocal(usize),
}

/// Empty struct that implements `ASTVisitor<IrChunk>`.
///
/// See `ASTVisitor<IrChunk>` and [`ASTVisitor`] for information.
pub struct Compiler;

impl visitors::GlobalDefVisitor<IrChunk> for Compiler {
    fn visit_globaldef(&mut self, name: &Keyword, value: &LocalAST) -> Result<IrChunk> {
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

    fn globalvar_expr(&mut self, name: &Keyword) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(Literal::Keyword(name.clone())), IrOp::Load])
    }

    fn localvar_expr(&mut self, index: usize) -> Result<IrChunk> {
        Ok(vec![IrOp::LoadLocal(index)])
    }

    /*fn var_expr(&mut self, k: &Keyword) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(Literal::Keyword(k.clone())), IrOp::Load])
    }*/

    fn application_expr(&mut self, f: &Rc<LocalAST>, args: &[LocalAST]) -> Result<IrChunk> {
        let mut chunk = vec![];

        for e in args.iter().rev() {
            let mut e_chunk = self.visit(e)?;
            chunk.append(&mut e_chunk);
        }

        let mut f_chunk = self.visit(f)?;
        chunk.append(&mut f_chunk);

        chunk.push(IrOp::CallArity(args.len()));

        Ok(chunk)
    }
}

impl visitors::LLASTVisitor<IrChunk> for Compiler {
    fn visit_local_function(
        &mut self,
        args: &[Keyword],
        body: &Rc<LocalAST>,
        entry: bool,
    ) -> Result<IrChunk> {
        let mut ir = self.visit(body)?;

        if entry {
            ir.push(IrOp::PopEnv);
        }

        ir.push(IrOp::Return);

        let mut arg_ir: IrChunk = args
            .iter()
            .enumerate()
            //.map(|k| vec![IrOp::Lit(Literal::Keyword(k.clone())), IrOp::Store])
            .map(|(i, _)| IrOp::StoreLocal(i))
            .collect();

        if entry {
            arg_ir.insert(0, IrOp::PushEnv);
        }

        arg_ir.append(&mut ir);

        if !entry {
            tail_call_optimization(&mut arg_ir);
        }

        Ok(arg_ir)
    }
}

/// Compiles a raw [ `AST` ] into an [ `IrChunk` ]. See [ `Compiler` ] for implementation.
pub fn compile(a: &local::LocalAST) -> Result<IrChunk> {
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
pub fn pack_compile_lifted(llast: &local::LocalLiftedAST) -> Result<Bytecode> {
    let mut code = Bytecode::new(vec![]);

    // allocate chunks first
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

    // load functions into the chunks
    /*for (id, function) in llast.functions.iter().enumerate() {
        let chunk = id;
        let is_entry = id == llast.entry;

        /*let mut ir = compile(&(*function).body)?;

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
        }*/

        pack(&arg_ir, &mut code, chunk, 0)?;
    }*/

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
    use crate::ast;
    use crate::parser;
    use crate::str_to_ast;
    use crate::vm::bytecode;
    use crate::vm::VM;
    use test::Bencher;

    use crate::ast::passes::function_lifter;

    fn run(s: &'static str) -> Result<Literal> {
        let lits = parser::parse(s)?;

        let mut vm = VM::new(bytecode::Bytecode::new(vec![vec![]]));

        let ast = ast::ast(&lits, vm.environment.peek()?)?;

        let code = pack_compile_lifted(&ast)?;

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

        pack_compile_lifted(&llast).unwrap()
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

        code.dissassemble();

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

    #[bench]
    fn bench_toolchain(b: &mut Bencher) {
        use test;
        b.iter(|| {
            let ast = str_to_ast("(def add1 (lambda (x) (let (y 1) (+ 1 1)))) (add1 5)").unwrap();

            let last = function_lifter::lift_functions(&ast).unwrap();

            let llast = local::pass(&last).unwrap();

            test::black_box(pack_compile_lifted(&llast).unwrap());
        })
    }
}
