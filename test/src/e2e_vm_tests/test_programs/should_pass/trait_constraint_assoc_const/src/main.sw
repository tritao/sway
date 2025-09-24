script;

use std::assert::assert;

pub trait A {
    const C: bool;
}

struct S<T> {}

impl A for bool {
    const C: bool = false;
}

impl<T> A for S<T>
where
    T: A,
{
    const C: bool = true;
}

fn main() {
    assert(<S<bool> as A>::C);
    assert(!<bool as A>::C);
}
