use std::rc::Rc;
use im::hashmap::HashMap;

use data;
use errors::*;

pub type Env = HashMap<String, Rc<data::Literal>>;

#[derive(Debug)]
pub struct EnvStack{
    envs: Vec<Env>
}

impl EnvStack {
    pub fn new() -> EnvStack {
        EnvStack {
            envs: vec![Env::new()]
        }
    }

    pub fn insert(&mut self, k: String, v: Rc<data::Literal>) -> Result<()> {
        self.envs.last_mut().ok_or("No envs to insert into")?.insert(k, v);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn easy_insert(&mut self, k: &str, v: data::Literal) -> Result<()> {
        self.insert(k.to_string(), Rc::new(v))
    }

    pub fn get(&self, k: &str) -> Result<Rc<data::Literal>> {
        match self.peek()?.get(k) {
            Some(r) => Ok(Rc::clone(r)),
            None => Err(format!("Binding not found for {:}", k).into()),
        }
    }

    pub fn peek(&self) -> Result<&Env> {
        self.envs.last().ok_or_else(|| "Env stack empty".into())
    }

    pub fn push(&mut self) {
        let n = match self.envs.last() {
            Some(e) => e.clone(),
            None => Env::new(),
        };

        self.envs.push(n);
    }

    pub fn pop(&mut self) -> Result<()> {
        self.envs.pop().ok_or("Attempted to pop empty environment stack")?;
        Ok(())
    }
}

// TODO probably use refcells

#[cfg(test)]
mod tests {
    use data::Literal;
    use environment::Env;
    use environment::EnvStack;
    use std::rc::Rc;

    #[test]
    fn test_env() {
        let n = |x| Rc::new(Literal::Number(x));
        let mut root = Env::new();

        let s1 = "test1".to_string();
        let s11 = "test1".to_string();

        let s2 = "test2".to_string();
        let s21 = "test2".to_string();

        assert!(root.get(&s1).is_none());

        root.insert(s1, n(0));

        assert_eq!(*root.get(&s11).unwrap(), n(0));

        let l1 = root.update(s2, n(1));

        assert_eq!(*l1.get(&s21).unwrap(), n(1));

        assert_eq!(*l1.get(&s11).unwrap(), n(0));

        assert_eq!(*root.get(&s11).unwrap(), n(0));
        assert!(root.get(&s21).is_none());
    }

    #[test]
    fn test_env_stack() {
        let n = |x| Rc::new(Literal::Number(x));

        let mut root = EnvStack::new();

        let s1 = "test1".to_string();
        let s11 = "test1".to_string();

        let s2 = "test2".to_string();
        let s21 = "test2".to_string();

        assert!(root.get(&s1).is_err());

        root.insert(s1, n(0));

        assert_eq!(root.get(&s11).unwrap(), n(0));

        root.push();

        root.insert(s2, n(1));

        assert_eq!(root.get(&s21).unwrap(), n(1));

        assert_eq!(root.get(&s11).unwrap(), n(0));

        root.pop();

        assert_eq!(root.get(&s11).unwrap(), n(0));
        assert!(root.get(&s21).is_err());
    }
}
