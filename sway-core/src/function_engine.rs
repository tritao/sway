mod engine;
use std::fmt::{Debug, Display};

/// A identifier to uniquely refer to our type terms
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct FunctionId(usize);

impl std::ops::Deref for FunctionId {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", *self))
    }
}

impl Debug for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", *self))
    }
}

impl From<usize> for FunctionId {
    fn from(o: usize) -> Self {
        FunctionId(o)
    }
}
