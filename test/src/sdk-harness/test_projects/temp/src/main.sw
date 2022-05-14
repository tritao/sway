contract;

pub struct MyStruct {
    value: u64,
}

impl MyStruct {
    fn some_func(self, a: u64) -> MyStruct {
        MyStruct {
            value: self.value + a
        }
    }

    fn some_other_func(self, b: u64) -> MyStruct {
        self.some_func(b)
    }

    // ^^^^^^^^^ No method named "some_func" found for type "struct MyStruct<u64>".
}

abi MyContract {
    fn test(a: MyStruct, b: u64) -> MyStruct;
}

impl MyContract for Contract {

    fn test(a : MyStruct, b: u64) -> MyStruct {
        
        let a = MyStruct {
            value: 13
        };

        a.some_func(b)
    }
}