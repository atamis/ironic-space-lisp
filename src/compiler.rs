use std::rc::Rc;

use ast::ASTVisitor;
use ast::Def;
use ast::AST;
use data::Keyword;
use data::Literal;
use errors::*;
use vm::Bytecode;
use vm::Chunk;
use vm::Op;

pub type IrChunk = Vec<IrOp>;

#[derive(Debug, PartialEq)]
pub enum IrOp {
    Lit(Literal),
    Return,
    Call,
    Jump(Rc<IrChunk>),
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
}

pub struct Compiler;

impl Compiler {
    fn visit_def(&mut self, def: &Def) -> Result<IrChunk> {
        let mut body_chunk = self.visit(&def.value)?;

        body_chunk.push(IrOp::Lit(Literal::Keyword(def.name.clone())));
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
        let mut chunk = self.visit_def(def)?;

        chunk.append(&mut self.var_expr(&def.name)?);

        Ok(chunk)
    }

    fn let_expr(&mut self, defs: &Vec<Def>, body: &Rc<AST>) -> Result<IrChunk> {
        let mut chunk = vec![IrOp::PushEnv];

        for d in defs {
            let mut def_chunk = self.visit_def(d)?;
            chunk.append(&mut def_chunk);
        }

        let mut body_chunk = self.visit(body)?;

        chunk.append(&mut body_chunk);

        chunk.push(IrOp::PopEnv);

        Ok(chunk)
    }

    fn do_expr(&mut self, exprs: &Vec<AST>) -> Result<IrChunk> {
        let mut chunk = vec![];

        for (idx, e) in exprs.iter().enumerate() {
            let mut e_chunk = self.visit(e)?;
            chunk.append(&mut e_chunk);

            // pop every interstitial value except the last
            if idx != (exprs.len() - 1) {
                chunk.push(IrOp::Pop);
            }
        }

        Ok(chunk)
    }

    fn lambda_expr(&mut self, _args: &Vec<Keyword>, _body: &Rc<AST>) -> Result<IrChunk> {
        Err(err_msg("Not implemented"))
    }

    fn var_expr(&mut self, k: &Keyword) -> Result<IrChunk> {
        Ok(vec![IrOp::Lit(Literal::Keyword(k.clone())), IrOp::Load])
    }

    fn application_expr(&mut self, f: &Rc<AST>, args: &Vec<AST>) -> Result<IrChunk> {
        let mut chunk = vec![];

        for e in args.iter().rev() {
            let mut e_chunk = self.visit(e)?;
            chunk.append(&mut e_chunk);
        }

        let mut f_chunk = self.visit(f)?;
        chunk.append(&mut f_chunk);

        chunk.push(IrOp::Call);

        Ok(chunk)
    }
}

pub fn compile(a: &AST) -> Result<IrChunk> {
    let mut c = Compiler {};
    c.visit(a)
}

fn alloc_chunk(code: &mut Bytecode) -> usize {
    let idx = code.chunks.len();
    code.chunks.push(Chunk { ops: vec![] });
    idx
}

pub fn pack_start(ir: &IrChunk) -> Result<Bytecode> {
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

pub fn pack(ir: &IrChunk, code: &mut Bytecode, chunk_idx: usize, op_idx: usize) -> Result<usize> {
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
            IrOp::JumpCond {pred, then, els} => {
                let els_idx = alloc_chunk(code);
                pack(els, code, els_idx, 0)?;

                let then_idx = alloc_chunk(code);
                pack(then, code, then_idx, 0)?;

                code.chunks[chunk_idx].ops.push(Op::Lit(Literal::Address((els_idx, 0))));
                op_idx += 1;
                code.chunks[chunk_idx].ops.push(Op::Lit(Literal::Address((then_idx, 0))));
                op_idx += 1;

                op_idx = pack(pred, code, chunk_idx, op_idx)?;

                let res_idx = op_idx + 1;
                let mut ret_code = vec![Op::Lit(Literal::Address((chunk_idx, res_idx))), Op::Jump];

                code.chunks[els_idx].ops.append(&mut ret_code.clone());
                code.chunks[then_idx].ops.append(&mut ret_code);

                Op::JumpCond
            },
            IrOp::Jump(sub) => {
                let sub_idx = alloc_chunk(code);
                pack(sub, code, sub_idx, 0)?;
                code.chunks[chunk_idx].ops.push(Op::Lit(Literal::Address((sub_idx, 0))));
                op_idx += 1;
                Op::Jump
            }
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
    use str_to_ast;
    use vm::VM;

    fn t(s: &'static str) -> Result<IrChunk> {
        let a = str_to_ast(s).unwrap().remove(0);

        compile(&a)
    }

    fn run(s: &'static str) -> Result<Literal> {
        let asts = str_to_ast(s)?;
        let ast = AST::Do(asts);

        let ir = compile(&ast)?;

        let code = pack_start(&ir)?;

        let mut vm = VM::new(code);

        vm.step_until_cost(10000).map(Option::unwrap)
    }

    //#[test]
    fn test_value() {
        assert_eq!(t("3").unwrap(), vec![IrOp::Lit(Literal::Number(3))]);

        println!("{:?}", t("(0 1 2 3 4)"));
        println!("{:?}", t("(def x 0)"));
        println!("{:?}", t("(let (x 0 y 1) (9 x y))"));

        let code = pack_start(&t("(let (x 2 y 3) (if 0 x y))").unwrap()).unwrap();
        code.dissassemble();

        let mut vm = VM::new(code);

        println!("{:?}", vm.step_until_value(true));

        assert!(false);
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
}
