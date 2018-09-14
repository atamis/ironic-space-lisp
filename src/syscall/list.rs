//! Holds list related syscalls

use data::Literal;
use errors::*;
use im::vector::Vector;
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
            ("len", Syscall::A1(Box::new(len))),
            ("cons", Syscall::A2(Box::new(cons))),
            ("car", Syscall::A1(Box::new(car))),
            ("cdr", Syscall::A1(Box::new(cdr))),
            ("first", Syscall::A1(Box::new(car))),
            ("rest", Syscall::A1(Box::new(cdr))),
            ("empty?", Syscall::A1(Box::new(empty))),
        ])
    }
}

fn len(a: Literal) -> Result<Literal> {
    Ok(Literal::Number(a.ensure_list()?.len() as u32))
}

// improper lists banned BTFO
fn cons(a: Literal, b: Literal) -> Result<Literal> {
    let mut lst = b.ensure_list()?;
    lst.push_front(a);
    Ok(Literal::List(lst))
}

fn car(a: Literal) -> Result<Literal> {
    let lst = a.ensure_list()?;

    match lst.len() {
        0 => Err(err_msg("Cannot car empty list")),
        _ => Ok(a.ensure_list()?.remove(0)),
    }
}

fn cdr(a: Literal) -> Result<Literal> {
    let lst = a.ensure_list()?;
    match lst.len() {
        0 => Err(err_msg("Cannot car empty list")),
        1 => Ok(Literal::List(Vector::new())),
        _ => {
            let (_, rest) = lst.split_at(1);

            Ok(Literal::List(rest))
        }
    }
}

fn empty(a: Literal) -> Result<Literal> {
    Ok(Literal::Boolean(a.ensure_list()?.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use data::list;

    #[test]
    fn test_len() {
        let lst = list(vec![
            Literal::Number(1),
            Literal::Number(2),
            Literal::Number(3),
        ]);

        assert_eq!(len(lst).unwrap(), Literal::Number(3));
    }

    #[test]
    fn test_cons() {
        let lst = list(vec![Literal::Number(2), Literal::Number(3)]);

        assert_eq!(
            cons(Literal::Number(1), lst).unwrap(),
            list(vec!(
                Literal::Number(1),
                Literal::Number(2),
                Literal::Number(3)
            ))
        );
    }

    #[test]
    fn test_car() {
        let lst = list(vec![
            Literal::Number(1),
            Literal::Number(2),
            Literal::Number(3),
        ]);

        assert_eq!(car(lst).unwrap(), Literal::Number(1));

        assert!(car(list(vec![])).is_err());
    }

    #[test]
    fn test_cdr() {
        let lst = list(vec![
            Literal::Number(1),
            Literal::Number(2),
            Literal::Number(3),
        ]);

        assert_eq!(
            cdr(lst).unwrap(),
            list(vec!(Literal::Number(2), Literal::Number(3)))
        );

        assert!(cdr(list(vec![])).is_err());

        assert_eq!(
            cdr(list(vec![Literal::Number(1)])).unwrap(),
            list(Vec::new())
        );
    }

    #[test]
    fn test_empty() {
        let lst = list(vec![]);
        assert_eq!(empty(lst).unwrap(), Literal::Boolean(true));

        let lst = list(vec![Literal::Number(1)]);
        assert_eq!(empty(lst).unwrap(), Literal::Boolean(false));
    }

}
