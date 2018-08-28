//! Holds math related syscalls

use data::Literal;
use errors::*;
use syscall::SyscallFactory;
use syscall::Syscall;
use syscall::destatic;

#[derive(Default)]
pub struct Factory;

impl Factory {
    pub fn new() -> Factory { Factory {} }
}

impl SyscallFactory for Factory {
    fn syscalls(&self) -> Vec<(String, Syscall)> {
        destatic(vec![
            ("+", Syscall::A2(Box::new(add))),
            ("-", Syscall::A2(Box::new(sub))),
            ("=", Syscall::A2(Box::new(eq))),
        ])
    }
}


fn add(a: Literal, b: Literal) -> Result<Literal> {
    Ok(Literal::Number(a.ensure_number()? + b.ensure_number()?))
}

fn sub(a: Literal, b: Literal) -> Result<Literal> {
    Ok(Literal::Number(a.ensure_number()? - b.ensure_number()?))
}

fn eq(a: Literal, b: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a == b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_math() {
        assert_eq!(add(Literal::Number(1), Literal::Number(1)).unwrap(), Literal::Number(2));
        assert!(add(Literal::Boolean(true), Literal::Number(1)).is_err());

        assert_eq!(sub(Literal::Number(1), Literal::Number(1)).unwrap(), Literal::Number(0));
        assert!(add(Literal::Boolean(true), Literal::Number(1)).is_err());

        assert_eq!(eq(Literal::Number(1), Literal::Number(1)).unwrap(), Literal::Boolean(true));
        assert_eq!(eq(Literal::Number(1), Literal::Number(0)).unwrap(), Literal::Boolean(false));
    }
}
