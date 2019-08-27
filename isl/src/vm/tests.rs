use super::*;
use test::Bencher;

#[test]
fn test_bytecode_errors() {
    let empty = Bytecode::new(vec![]);
    assert!(empty.addr((0, 0)).is_err());

    let single = Bytecode::new(vec![vec![Op::Return]]);
    let maybe_ret = single.addr((0, 0));
    assert!(maybe_ret.is_ok());
    assert_eq!(maybe_ret.unwrap(), Op::Return);
    assert!(single.addr((0, 1)).is_err());
    assert!(single.addr((1, 0)).is_err());
}

#[test]
fn test_pcounter() {
    let single = Bytecode::new(vec![vec![Op::Return]]);

    let mut vm = VM::new(single);

    let a = vm.pcounter();
    assert!(a.is_ok());
    assert_eq!(a.unwrap(), (0, 0));

    let b = vm.pcounter();
    assert!(b.is_ok());
    assert_eq!(b.unwrap(), (0, 1));

    vm.frames.pop().unwrap();

    assert!(vm.pcounter().is_err());
}

#[test]
fn test_jump() {
    let single = Bytecode::new(vec![vec![Op::Return]]);
    let mut vm = VM::new(single);

    vm.jump((5, 5)).unwrap();
    assert_eq!(vm.frames.last().unwrap().addr, (5, 5));
}

#[test]
fn test_op_lit() {
    let empty = Bytecode::new(vec![vec![]]);
    let mut vm = VM::new(empty);

    vm.op_lit(Literal::Number(0)).unwrap();
    assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0))
}

#[test]
fn test_op_return() {
    let empty = Bytecode::new(vec![vec![]]);
    let mut vm = VM::new(empty);

    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_return().unwrap();
    assert!(vm.frames.is_empty());
}

#[test]
fn test_op_call() {
    let empty = Bytecode::new(vec![vec![]]);
    let mut vm = VM::new(empty);

    vm.op_lit(Literal::Number(0)).unwrap();
    assert!(vm.op_call().is_err());
    assert!(vm.stack.is_empty()); // only going to test this once

    vm.op_lit(Literal::Address((0, 0))).unwrap();
    assert!(vm.op_call().is_ok());
    assert_eq!(vm.frames.last().unwrap().addr, (0, 0));
    assert_eq!(vm.frames.len(), 2)
}

#[test]
fn test_op_jump() {
    let empty = Bytecode::new(vec![vec![]]);
    let mut vm = VM::new(empty);

    vm.op_lit(Literal::Number(0)).unwrap();
    assert!(vm.op_jump().is_err());

    vm.op_lit(Literal::Address((5, 5))).unwrap();
    assert!(vm.op_jump().is_ok());
    assert_eq!(vm.frames.last().unwrap().addr, (5, 5));
}

#[test]
fn test_jumpcond_then() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Address((6, 0))).unwrap();
    vm.op_lit(Literal::Address((5, 0))).unwrap();
    vm.op_lit(Literal::Boolean(true)).unwrap();
    assert!(vm.op_jumpcond().is_ok());
    assert_eq!(vm.frames.last().unwrap().addr, (5, 0));
}

#[test]
fn test_jumpcond_else() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Address((6, 0))).unwrap();
    vm.op_lit(Literal::Address((5, 0))).unwrap();
    vm.op_lit(Literal::Boolean(false)).unwrap();
    assert!(vm.op_jumpcond().is_ok());
    assert_eq!(vm.frames.last().unwrap().addr, (6, 0));
}

#[test]
fn test_jumpcond_errors() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_lit(Literal::Address((5, 0))).unwrap();
    vm.op_lit(Literal::Boolean(false)).unwrap();
    assert!(vm.op_jumpcond().is_err());

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Address((6, 0))).unwrap();
    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_lit(Literal::Boolean(false)).unwrap();
    assert!(vm.op_jumpcond().is_err());

    // Now uses Literal::truthy, which is defined for all values.
    /*let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Address((6, 0))).unwrap();
    vm.op_lit(Literal::Address((5, 0))).unwrap();
    vm.op_lit(Literal::Number(1)).unwrap();
    assert!(vm.op_jumpcond().is_err());*/
}

#[test]
fn test_op_load() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    assert!(vm.environment.get("test").is_err());
    vm.environment.insert("test".to_string(), 0.into()).unwrap();
    assert_eq!(*vm.environment.get("test").unwrap(), Literal::Number(0));
    vm.op_lit(Literal::Keyword("test".to_string())).unwrap();
    vm.op_load().unwrap();
    assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));
}

#[test]
fn test_op_store() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    assert!(vm.environment.get("test").is_err());
    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_lit(Literal::Keyword("test".to_string())).unwrap();
    vm.op_store().unwrap();
    assert_eq!(*vm.environment.get("test").unwrap(), Literal::Number(0));
}

#[test]
fn test_op_pushenv_popenv() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.environment
        .insert("test1".to_string(), 0.into())
        .unwrap();
    assert!(vm.environment.get("test2").is_err());

    vm.op_pushenv().unwrap();

    assert_eq!(*vm.environment.get("test1").unwrap(), Literal::Number(0));

    vm.environment
        .insert("test2".to_string(), 1.into())
        .unwrap();
    assert_eq!(*vm.environment.get("test2").unwrap(), Literal::Number(1));
    vm.op_lit(Literal::Keyword("test1".to_string())).unwrap();
    vm.op_load().unwrap();
    assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));

    vm.op_popenv().unwrap();
    assert_eq!(*vm.environment.get("test1").unwrap(), Literal::Number(0));
    assert!(vm.environment.get("test2").is_err());
}

#[test]
fn test_op_dup() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));
    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_dup().unwrap();

    assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));
    vm.stack.pop().unwrap();
    assert_eq!(*vm.stack.last().unwrap(), Literal::Number(0));

    vm.stack.pop().unwrap(); // empty the stack

    assert!(vm.op_dup().is_err());
}

#[test]
fn test_op_pop() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));
    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_pop().unwrap();

    assert_eq!(vm.stack.len(), 0);

    assert!(vm.op_pop().is_err());
}

#[test]
fn test_op_make_closure() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));
    vm.op_lit(Literal::Address((0, 0))).unwrap();
    vm.op_lit(Literal::Number(0)).unwrap();
    vm.op_make_closure().unwrap();

    assert_eq!(*vm.stack.last().unwrap(), Literal::Closure(0, (0, 0)));
}

#[test]
fn test_op_call_closure() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));
    vm.op_lit(Literal::Closure(2, (0, 0))).unwrap();
    vm.op_call().unwrap();

    assert_eq!(vm.frames.last().unwrap().addr, (0, 0));
    assert_eq!(vm.frames.len(), 2)
}

#[test]
fn test_op_call_arity() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Closure(2, (0, 0))).unwrap();
    assert!(vm.op_call_arity(2).is_ok());

    assert_eq!(vm.frames.last().unwrap().addr, (0, 0));

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Closure(2, (0, 0))).unwrap();
    assert!(vm.op_call_arity(1).is_err());

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Address((0, 0))).unwrap();
    assert!(vm.op_call_arity(1).is_ok());

    assert_eq!(vm.frames.last().unwrap().addr, (0, 0));

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(Literal::Address((0, 0))).unwrap();
    assert!(vm.op_call_arity(2).is_ok());

    assert_eq!(vm.frames.last().unwrap().addr, (0, 0));
}

#[test]
fn test_wait() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));
    vm.state = VMState::Running;

    vm.op_wait().unwrap();

    assert_eq!(vm.state, VMState::Waiting);
    assert!(!vm.state.can_run());

    vm.answer_waiting(1.into()).unwrap();
}

#[test]
fn test_pid() {
    use crate::exec;
    use crate::exec::ExecHandle;
    use crate::futures::StreamExt;
    use futures::channel::mpsc;
    use futures::executor;

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_pid().unwrap();

    assert_eq!(*vm.stack.last().unwrap(), false.into());

    let (tx, mut rx) = mpsc::channel::<exec::RouterMessage>(10);

    let mut handler = exec::RouterHandle::new(tx);
    let pid = handler.get_pid();

    vm.proc = Some(Box::new(handler));

    vm.op_pid().unwrap();

    assert_eq!(*vm.stack.last().unwrap(), pid.into());

    let reg_msg = executor::block_on(rx.next()).unwrap();

    if let exec::RouterMessage::Register(p, _) = reg_msg {
        assert_eq!(p, pid);
    } else {
        panic!();
    }
}

#[test]
fn test_send() {
    use crate::exec;
    use crate::exec::ExecHandle;
    use futures::channel::mpsc;
    use futures::executor;
    use tokio::prelude::*;

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    let (tx, mut rx) = mpsc::channel::<exec::RouterMessage>(10);

    let mut handler = exec::RouterHandle::new(tx);
    let pid = handler.get_pid();

    vm.proc = Some(Box::new(handler));

    vm.op_lit("test-message".into()).unwrap();
    vm.op_pid().unwrap();
    vm.op_send().unwrap();

    // Throw out register message
    executor::block_on(rx.next()).unwrap();
    let msg = executor::block_on(rx.next()).unwrap();

    if let exec::RouterMessage::Send(p, lit) = msg {
        assert_eq!(lit, "test-message".into());
        assert_eq!(p, pid);
    } else {
        eprintln!("{:?}, {:?}", pid, msg);
        panic!();
    }
}

#[test]
fn test_fork() {
    use crate::exec;
    use futures::future;

    let exec = exec::Exec::new();

    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    let handler = exec.get_handle();

    vm.proc = Some(Box::new(handler));

    exec.runtime
        .block_on(future::lazy(|_| vm.op_fork()))
        .unwrap();
}

#[test]
fn test_fork2() {
    use crate::exec;
    use crate::exec::ExecHandle;
    use std::time::Duration;
    use tokio::timer::Timeout;

    let dur = Duration::from_millis(1000);

    let mut exec = exec::Exec::new();

    let mut test_handler = exec.get_handle();

    let code = Bytecode::new(vec![vec![
        Op::Fork,
        Op::Dup,
        //Op::Lit("print".into()),
        //Op::Load,
        //Op::CallArity(1),
        Op::Lit(test_handler.get_pid().into()),
        Op::Send,
        Op::Pop,
        Op::Return,
    ]]);

    let vm = VM::new(Bytecode::new(vec![vec![]]));

    assert_eq!(exec.sched(vm, &code).1.unwrap(), false.into());

    let mut ans = vec![];

    ans.push(
        exec.runtime
            .block_on(Timeout::new(test_handler.receive(), dur))
            .unwrap()
            .unwrap(),
    );

    ans.push(
        exec.runtime
            .block_on(Timeout::new(test_handler.receive(), dur))
            .unwrap()
            .unwrap(),
    );

    println!("{:?}", ans);

    assert!(ans.contains(&true.into()));
    assert!(ans.contains(&false.into()));

    //exec.wait();
}

#[test]
fn test_store_local() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(1.into()).unwrap();
    vm.op_store_local(0).unwrap();

    assert_eq!(vm.frames.last().unwrap().locals[0], 1.into());

    vm.op_lit(2.into()).unwrap();
    vm.op_store_local(1).unwrap();

    assert_eq!(vm.frames.last().unwrap().locals[1], 2.into());
}

#[test]
fn test_load_local() {
    let mut vm = VM::new(Bytecode::new(vec![vec![]]));

    vm.op_lit(1.into()).unwrap();
    vm.op_store_local(0).unwrap();
    vm.op_load_local(0).unwrap();

    assert_eq!(*vm.stack.last().unwrap(), Literal::from(1));

    vm.op_lit(2.into()).unwrap();
    vm.op_store_local(1).unwrap();
    vm.op_load_local(1).unwrap();

    assert_eq!(*vm.stack.last().unwrap(), Literal::from(2));
}

#[test]
fn test_step_until() {
    let mut ret = VM::new(Bytecode::new(vec![vec![Op::Return]]));
    assert!(ret.step_until_value().is_err());

    let mut ret = VM::new(Bytecode::new(vec![vec![
        Op::Lit(Literal::Number(0)),
        Op::Return,
    ]]));

    assert_eq!(ret.step_until_value().unwrap(), Literal::Number(0));

    // lol
    /*let mut never = VM::new(Bytecode::new(vec![vec![Op::Lit(Literal::Address((0, 0))),
                                                  Op::Jump,
                                                  Op::Return]]));
    assert_never_terminates!(never.step_until_value());*/

    //let mut empty = VM::new(Bytecode::new(vec![vec![]]));
    assert!(ret.step_until_value().is_err());
}

#[test]
fn test_step_until_cost() {
    let mut ret = VM::new(Bytecode::new(vec![vec![
        Op::Lit(Literal::Number(0)),
        Op::Return,
    ]]));

    let res = ret.step_until_cost(0);
    println!("{:?}", res);

    assert!(res.is_ok());
    assert!(res.unwrap().is_none());

    let res = ret.step_until_cost(50);

    assert!(res.is_ok());
    assert_eq!(res.unwrap().unwrap(), Literal::Number(0));

    let res = ret.step_until_cost(50);

    assert!(res.is_err());

    let mut ret = VM::new(Bytecode::new(vec![vec![
        Op::Lit(Literal::Number(0)),
        Op::Return,
    ]]));

    // Partial
    let res = ret.step_until_cost(7);

    assert!(res.is_ok());
    assert!(res.unwrap().is_none());
}

#[test]
fn test_step_until_value_waiting() {
    let mut vm = VM::new(Bytecode::new(vec![vec![Op::Wait, Op::Return]]));
    assert_eq!(vm.step_until_cost(10000).unwrap(), None);
    assert_eq!(vm.state, VMState::Waiting);
    vm.answer_waiting(1.into()).unwrap();
    assert_eq!(vm.step_until_cost(10000).unwrap(), Some(1.into()));
    assert_eq!(vm.state, VMState::Done(1.into()));
    assert!(vm.answer_waiting(false.into()).is_err());
}

#[test]
fn test_syscalls() {
    let mut vm = VM::new(Bytecode::new(vec![vec![
        Op::Lit(Literal::Number(1)),
        Op::Lit(Literal::Number(1)),
        Op::Lit(Literal::Keyword("+".to_string())),
        Op::Load,
        Op::Call,
        Op::Return,
    ]]));

    assert_eq!(
        vm.step_until_cost(10000).unwrap().unwrap(),
        Literal::Number(2)
    );
}

// Benchmarks

#[bench]
fn bench_nested_envs(b: &mut Bencher) {
    
    
    
    use crate::ast;
    use crate::compiler::pack_compile_lifted;
    use crate::parser;

    let s = "(let (x 0) (let (y 1) (let (z 2) x)))";
    let lits = parser::parse(&s).unwrap();

    let mut vm = VM::new(bytecode::Bytecode::new(vec![]));

    let ast = ast::ast(&lits, vm.environment.peek().unwrap()).unwrap();

    let code = pack_compile_lifted(&ast).unwrap();

    vm.import_jump(&code);

    vm.code.dissassemble();

    vm.step_until_cost(10000).unwrap().unwrap();

    b.iter(|| {
        vm.frames.push(Frame::new((0, 0)));
        vm.step_until_cost(10000).unwrap().unwrap();
    })
}

#[bench]
fn bench_infinite_recursion(b: &mut Bencher) {
    
    use crate::compiler;
    
    use crate::ast;
    
    use crate::parser;

    let s = "(def x (lambda () (x))) (x)";

    let lits = parser::parse(&s).unwrap();

    let vm = VM::new(bytecode::Bytecode::new(vec![vec![]]));

    let ast = ast::ast(&lits, vm.environment.peek().unwrap()).unwrap();

    let code = compiler::pack_compile_lifted(&ast).unwrap();

    code.dissassemble();

    let mut vm = VM::new(code);

    b.iter(|| {
        vm.frames.clear();
        vm.frames.push(Frame::new((0, 0)));
        vm.step_until_cost(10000).unwrap();
    });
}

#[bench]
fn bench_op_lit(b: &mut Bencher) {
    b.iter(|| {
        VM::new(Bytecode::new(vec![]))
            .op_lit(Literal::Number(0))
            .unwrap()
    })
}

#[bench]
fn bench_op_ret(b: &mut Bencher) {
    b.iter(|| {
        let mut vm = VM::new(Bytecode::new(vec![]));
        vm.op_lit(Literal::Number(0)).unwrap();
        vm.op_return().unwrap();
    })
}

#[bench]
fn bench_op_call(b: &mut Bencher) {
    b.iter(|| {
        let mut vm = VM::new(Bytecode::new(vec![]));
        vm.op_lit(Literal::Address((0, 0))).unwrap();
        vm.op_call().unwrap();
    })
}

#[bench]
fn bench_op_jump(b: &mut Bencher) {
    b.iter(|| {
        let mut vm = VM::new(Bytecode::new(vec![]));
        vm.op_lit(Literal::Address((0, 0))).unwrap();
        vm.op_jump().unwrap();
    })
}

#[bench]
fn bench_op_jumpcond(b: &mut Bencher) {
    b.iter(|| {
        let mut vm = VM::new(Bytecode::new(vec![]));
        vm.op_lit(Literal::Address((0, 0))).unwrap();
        vm.op_lit(Literal::Address((0, 0))).unwrap();
        vm.op_lit(Literal::Boolean(true)).unwrap();
        vm.op_jumpcond().unwrap();
    })
}

// Bytecode

#[test]
fn test_bytecode_import() {
    let a = |a1, a2| Op::Lit(Literal::Address((a1, a2)));

    let mut b1 = Bytecode::new(vec![vec![a(0, 0), a(1, 3)], vec![a(1, 0), a(0, 3)]]);

    let b2 = Bytecode::new(vec![vec![a(0, 0), a(1, 3)], vec![a(1, 0), a(0, 3)]]);

    let b3 = Bytecode::new(vec![
        vec![a(0, 0), a(1, 3)],
        vec![a(1, 0), a(0, 3)],
        vec![a(2, 0), a(3, 3)],
        vec![a(3, 0), a(2, 3)],
    ]);

    b1.import(&b2);

    assert_eq!(b1, b3);
}
