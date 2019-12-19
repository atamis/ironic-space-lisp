//! Holds list related syscalls

use crate::data::Literal;
use crate::errors::*;
use crate::syscall::destatic;
use crate::syscall::Syscall;
use crate::syscall::SyscallFactory;
use im::vector::Vector;

/// A `list` syscall factory.
#[derive(Default)]
pub struct Factory;

impl Factory {
    /// Create a `list` syscall factory.
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
            ("nth", Syscall::A2(Box::new(n))),
            ("append", Syscall::A2(Box::new(append))),
        ])
    }
}

fn len(a: Literal) -> Result<Literal> {
    Ok(Literal::Number(a.ensure_list()?.len() as i64))
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
        0 => Err(err_msg("Cannot cdr empty list")),
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

fn n(a: Literal, b: Literal) -> Result<Literal> {
    let a = a.ensure_number()?;
    let b = b.ensure_list()?;

    let nth = b
        .get(a as usize)
        .ok_or_else(|| format_err!("Index out of bounds {:}", a))?;
    Ok(nth.clone())
}

fn append(a: Literal, b: Literal) -> Result<Literal> {
    let mut a = a.ensure_list()?;
    let b = b.ensure_list()?;

    a.append(b);

    Ok(Literal::List(a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::list;

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

    #[test]
    fn test_n() {
        let lst = list(vec![
            Literal::Number(1),
            Literal::Number(2),
            Literal::Number(3),
        ]);

        assert_eq!(
            n(Literal::Number(0), lst.clone()).unwrap(),
            Literal::Number(1)
        );
        assert_eq!(
            n(Literal::Number(1), lst.clone()).unwrap(),
            Literal::Number(2)
        );
        assert_eq!(
            n(Literal::Number(2), lst.clone()).unwrap(),
            Literal::Number(3)
        );
    }

    #[test]
    fn test_append() {
        let lst1 = list(vec![
            Literal::Number(1),
            Literal::Number(2),
            Literal::Number(3),
        ]);

        let lst2 = list(vec![
            Literal::Number(4),
            Literal::Number(5),
            Literal::Number(6),
        ]);

        let lst3 = list(vec![
            Literal::Number(1),
            Literal::Number(2),
            Literal::Number(3),
            Literal::Number(4),
            Literal::Number(5),
            Literal::Number(6),
        ]);

        assert_eq!(append(lst1.clone(), lst2).unwrap(), lst3);

        assert_eq!(append(lst1.clone(), list(vec![])).unwrap(), lst1);
    }
}
