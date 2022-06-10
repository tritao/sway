script;

use core::*;
use std::assert::assert;

struct Data<T> {
    value: T,
}

impl<T> Data<T> {
    fn new(value: T) -> Self {
        Data {
            value
        }
    }
}

fn main() -> bool {
    let value = 1;
    let data = Data {
        value
    };
    assert(data.value == value);

    let data = ~Data::new::<u64>(value);
    assert(data.value == value);

    true
}
