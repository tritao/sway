contract;

//use storage_access_abi::{S, T, StorageAccess};
use std::constants::NATIVE_ASSET_ID;

pub struct S {
    x: u64,
    y: u64,
    z: b256,
    t: T
}

pub struct T {
    x: u64,
    y: u64,
    z: b256
}

abi StorageAccess {
    fn get_x_0() -> u32; 
}

storage {
    s: S
}

impl StorageAccess for Contract {
    fn get_x_0() -> u32 {
        let y = storage.s.t.x;
        0
    }
}
