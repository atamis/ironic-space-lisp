//! Runtime size of data values
use data;
use env::EnvStack;
use im::Vector;
use std::mem::size_of;
use vm;

/// Express size in "runtime" size.
///
/// This is a slightly nebulous concept, but it counts the size of
/// accessible runtime values/objects, ignoring values that don't
/// "count", like the size of the compiled bytecode.
pub trait DataSize {
    /// Size in bytes
    fn data_size(&self) -> usize;
}

impl DataSize for data::Literal {
    fn data_size(&self) -> usize {
        size_of::<data::Literal>()
            + if let data::Literal::List(v) = self {
                v.data_size()
            } else {
                0
            }
    }
}

impl<T> DataSize for Vec<T>
where
    T: DataSize,
{
    fn data_size(&self) -> usize {
        self.iter().map(DataSize::data_size).sum()
    }
}

impl<T> DataSize for Vector<T>
where
    T: DataSize + Clone,
{
    fn data_size(&self) -> usize {
        self.iter().map(DataSize::data_size).sum()
    }
}

impl DataSize for EnvStack {
    fn data_size(&self) -> usize {
        match self.peek() {
            Ok(env) => env
                .iter()
                .map(|(s, lit)| s.data_size() + lit.data_size())
                .sum(),
            Err(_) => 0,
        }
    }
}

impl DataSize for vm::VM {
    fn data_size(&self) -> usize {
        self.stack.data_size() + self.environment.data_size()
    }
}

impl DataSize for String {
    fn data_size(&self) -> usize {
        self.len()
    }
}
