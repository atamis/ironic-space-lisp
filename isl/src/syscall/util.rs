//! Holds general utility syscalls.
//!
//! Registers syscalls `list?, Symbol?, print, or, and, even?, odd?, error`
use crate::data::Literal;
use crate::errors::*;
use crate::syscall::destatic;
use crate::syscall::Syscall;
use crate::syscall::SyscallFactory;

/// A `util` syscall factory.
#[derive(Default)]
pub struct Factory;

impl Factory {
    /// Create a `util` syscall factory.
    pub fn new() -> Factory {
        Factory {}
    }
}

impl SyscallFactory for Factory {
    fn syscalls(&self) -> Vec<(String, Syscall)> {
        destatic(vec![
            ("list?", Syscall::A1(Box::new(list_pred))),
            ("symbol?", Syscall::A1(Box::new(symbol_pred))),
            ("print", Syscall::A1(Box::new(println))),
            ("or", Syscall::A2(Box::new(or))),
            ("and", Syscall::A2(Box::new(and))),
            ("even?", Syscall::A1(Box::new(even_pred))),
            ("odd?", Syscall::A1(Box::new(odd_pred))),
            ("error", Syscall::A1(Box::new(vm_error))),
            ("size", Syscall::A1(Box::new(size))),
        ])
    }
}

fn list_pred(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.is_list()))
}

fn symbol_pred(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.is_symbol()))
}

fn println(a: Literal) -> Result<Literal> {
    println!("{:?}", a);
    Ok(a)
}

fn or(a: Literal, b: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.ensure_bool()? || b.ensure_bool()?))
}

fn and(a: Literal, b: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.ensure_bool()? && b.ensure_bool()?))
}

fn even_pred(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.ensure_number()? % 2 == 0))
}

fn odd_pred(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.ensure_number()? % 2 == 1))
}

fn vm_error(a: Literal) -> Result<Literal> {
    Err(format_err!("Runtime error: {:?}", a))
}

fn size(a: Literal) -> Result<Literal> {
    use crate::size::DataSize;
    Ok(Literal::Number(a.data_size() as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::list;

    fn mytrue() -> Literal {
        Literal::Boolean(true)
    }
    fn myfalse() -> Literal {
        Literal::Boolean(false)
    }

    #[test]
    fn test_list_pred() {
        assert_eq!(list_pred(list(vec![])).unwrap(), mytrue());
        assert_eq!(list_pred(list(vec![Literal::Number(1)])).unwrap(), mytrue());
        assert_eq!(list_pred(Literal::Number(1)).unwrap(), myfalse());
    }

    #[test]
    fn test_symbol_pred() {
        assert_eq!(
            symbol_pred(Literal::Symbol("test".to_string())).unwrap(),
            mytrue()
        );
        assert_eq!(symbol_pred(Literal::Number(1)).unwrap(), myfalse());
    }

    #[test]
    fn test_or() {
        assert_eq!(or(mytrue(), mytrue()).unwrap(), mytrue());
        assert_eq!(or(mytrue(), myfalse()).unwrap(), mytrue());
        assert_eq!(or(myfalse(), mytrue()).unwrap(), mytrue());
        assert_eq!(or(myfalse(), myfalse()).unwrap(), myfalse());
    }

    #[test]
    fn test_even_odd() {
        assert_eq!(even_pred(Literal::Number(1)).unwrap(), myfalse());
        assert_eq!(even_pred(Literal::Number(2)).unwrap(), mytrue());
        assert_eq!(odd_pred(Literal::Number(1)).unwrap(), mytrue());
        assert_eq!(odd_pred(Literal::Number(2)).unwrap(), myfalse());
    }
}
