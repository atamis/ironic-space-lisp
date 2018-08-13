
use std::rc::Rc;


#[derive(Debug, PartialEq, Eq)]
pub enum Lisp {
    Keyword(String),
    Num(i32),
    List(Rc<Vec<Lisp>>),
}

pub fn list(v: Vec<Lisp>) -> Lisp {
    Lisp::List(Rc::new(v))
}
