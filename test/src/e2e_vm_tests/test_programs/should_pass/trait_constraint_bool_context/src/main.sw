script;

trait A {
    fn f() -> bool;
}

impl A for u64 {
    fn f() -> bool {
        true
    }
}

impl A for bool {
    fn f() -> bool {
        false
    }
}

fn ff<T>() -> bool
where
    T: A,
{
    let v: bool = T::f();
    v
}

fn main() -> bool {
    ff::<u64>()
}
