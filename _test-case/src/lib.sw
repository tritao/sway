library;

trait MyTrait {
    fn f(self) -> bool;
} {
    fn f2(self) -> bool {
        self.f()
    }
}

trait MyTrait2: MyTrait {
}

impl<T1> MyTrait for (T1,)
where
    T1: MyTrait2,
{
    fn f(self) -> bool { self.0.f() }
} 
