//! Runtime environments
//!
//! This leverages immutable [`HashMap`]s from the [`im`](im) crate.
use im::hashmap::HashMap;
use std::rc::Rc;

use data;
use errors::*;

/// Represents runtime variable bindings.
///
/// Currently maintaints [`Rc`] pointers to the [`Literal`](data::Literal),
/// but this isn't necessary.
pub type Env = HashMap<String, Rc<data::Literal>>;

/// Represents multiple nested environment bindings.
#[derive(Debug, Default)]
pub struct EnvStack {
    envs: Vec<Env>,
}

impl EnvStack {
    pub fn new() -> EnvStack {
        EnvStack {
            envs: vec![Env::new()],
        }
    }

    /// Insert a new `(k, v)` pair into the top environment.
    pub fn insert(&mut self, k: String, v: Rc<data::Literal>) -> Result<()> {
        self.envs
            .last_mut()
            .ok_or_else(|| err_msg("No envs to insert into"))?
            .insert(k, v);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn easy_insert(&mut self, k: &str, v: data::Literal) -> Result<()> {
        self.insert(k.to_string(), Rc::new(v))
    }

    /// Get the value associated with a key. Returns `Err()` if not found.
    pub fn get(&self, k: &str) -> Result<Rc<data::Literal>> {
        match self.peek()?.get(k) {
            Some(r) => Ok(Rc::clone(r)),
            None => Err(format_err!("Binding not found for {:}", k)),
        }
    }

    /// Peek the top [`Env`] from the stack.
    pub fn peek(&self) -> Result<&Env> {
        self.envs.last().ok_or_else(|| err_msg("Env stack empty"))
    }

    /// Push a new local binding environment to the environment stack.
    pub fn push(&mut self) {
        let n = match self.envs.last() {
            Some(e) => e.clone(),
            None => Env::new(),
        };

        self.envs.push(n);
    }

    /// Pop the top environment, forgetting those local bindings.
    pub fn pop(&mut self) -> Result<()> {
        self.envs
            .pop()
            .ok_or_else(|| err_msg("Attempted to pop empty environment stack"))?;
        Ok(())
    }

    /// A vector of deduped envs. WARNING: this clones everything.
    ///
    /// Although nested [`Env`]s share data when the [`EnvStack`] is pushed
    /// and popped, each [`Env`] prints the entire stack regardless of whether
    /// that data is local to it or not. This dedups them for pretty printing,
    /// but it's very expensive.
    pub fn diff_stack(&self) -> Vec<Env> {
        let mut denvs = Vec::with_capacity(self.envs.len());
        let (first, rest) = self.envs.split_at(1);

        denvs.push(first[0].clone());

        for (idx, e) in rest.iter().enumerate() {
            // idx is the idx of the env - 1, because of split_at

            let last = { self.envs[idx].clone() };

            denvs.push(last.difference(e.clone()));
        }

        denvs
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

        root.insert(s1, n(0)).unwrap();

        assert_eq!(root.get(&s11).unwrap(), n(0));

        root.push();

        root.insert(s2, n(1)).unwrap();

        assert_eq!(root.get(&s21).unwrap(), n(1));

        assert_eq!(root.get(&s11).unwrap(), n(0));

        root.pop().unwrap();

        assert_eq!(root.get(&s11).unwrap(), n(0));
        assert!(root.get(&s21).is_err());
    }

    fn n(n: u32) -> Rc<Literal> {
        Rc::new(Literal::Number(n))
    }

    #[test]
    fn test_diff_stack() {
        let mut e = EnvStack::new();

        e.insert("test0".to_string(), n(0)).unwrap();
        e.push();
        e.insert("test1".to_string(), n(1)).unwrap();
        e.insert("test2".to_string(), n(2)).unwrap();
        e.push();
        e.push();
        e.insert("test3".to_string(), n(3)).unwrap();

        let ds = e.diff_stack();

        assert_eq!(ds[0], hashmap!{"test0".to_string() => n(0)});
        assert_eq!(
            ds[1],
            hashmap!{"test1".to_string() => n(1), "test2".to_string() => n(2)}
        );
        assert_eq!(ds[2], hashmap!{});
        assert_eq!(ds[3], hashmap!{"test3".to_string() => n(3)});

        assert_eq!(EnvStack::new().diff_stack(), [hashmap!{}]);
    }
}
