use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

use data;
use errors::*;

#[derive(Debug)]
pub struct Environment {
    bindings: HashMap<String, Rc<data::Literal>>,
    parent: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Environment {
        Environment {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn put(&mut self, k: String, v: Rc<data::Literal>) {
        self.bindings.insert(k, v);
    }

    pub fn get(&self, k: &String) -> Result<Rc<data::Literal>> {
        if let Some(v) = self.bindings.get(k) {
            return Ok(Rc::clone(v));
        }

        if let Some(ref p) = self.parent {
            return p.get(k);
        }

        Err(format!("Binding not found for {:}", k).into())
    }

    pub fn pop(&mut self) -> Result<()> {
        let parent = mem::replace(&mut self.parent, None);
        let parent = parent.ok_or("Attempted to pop root environment")?;

        *self = *parent;
        Ok(())
    }

    pub fn push(&mut self) {
        let n = Environment::new();
        let p = mem::replace(self, n);
        self.parent = Some(Box::new(p));
    }
}

#[cfg(test)]
mod tests {
    use data::Literal;
    use environment::Environment;
    use std::rc::Rc;

    #[test]
    fn test_environment() {
        let n = |x| Rc::new(Literal::Number(x));
        let mut e = Environment::new();

        let s1 = "test1".to_string();
        let s11 = "test1".to_string();

        let s2 = "test2".to_string();
        let s21 = "test2".to_string();

        assert!(e.get(&s1).is_err());

        e.put(s1, n(0));

        assert_eq!(e.get(&s11).unwrap(), n(0));

        e.push();

        assert_eq!(e.get(&s11).unwrap(), n(0));

        e.put(s2, n(1));

        assert_eq!(e.get(&s21).unwrap(), n(1));

        assert!(e.pop().is_ok());

        assert_eq!(e.get(&s11).unwrap(), n(0));
        assert!(e.get(&s21).is_err());

        assert!(e.pop().is_err());

    }
}
