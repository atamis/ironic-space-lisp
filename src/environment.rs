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
        let mut n = Environment::new();
        let p = mem::replace(self, n);
        n.parent = Some(Box::new(p));
    }
}
