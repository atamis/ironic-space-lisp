use data::Literal;
use errors::*;
use syscall::destatic;
use syscall::Syscall;
use syscall::SyscallFactory;

#[derive(Default)]
pub struct Factory;

impl Factory {
    pub fn new() -> Factory {
        Factory {}
    }
}

impl SyscallFactory for Factory {
    fn syscalls(&self) -> Vec<(String, Syscall)> {
        destatic(vec![
            ("list?", Syscall::A1(Box::new(list_pred))),
            ("keyword?", Syscall::A1(Box::new(keyword_pred))),
        ])
    }
}

fn list_pred(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.is_list()))
}

fn keyword_pred(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.is_keyword()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use data::list;

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
    fn test_keyword_pred() {}
}
