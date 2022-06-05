//! Functionality for loading code from external contracts an
library load_code;

use ::contract_id::ContractId;

/// Load an external contract `target`.
pub fn load_external_contract(target: ContractId) {
    // let BYTES: u64 = get_contract_size(target);
    asm(id: target, bytecode_ptr: 0, r3: get_contract_size(target)) {
        ldc r1 r2 r3;
    };
}

/// Load a portion of an external contract `target`.
/// The offset from the beginning of the contract is given by`offset`.
/// The length of the fragment to load is given by `bytes`.
pub fn load_external_fragment(target: ContractId, offset: u64, bytes: u64) {
    asm(id: target, offset: offset, r3: bytes) {
        ldc r1 r2 r3;
    };
}

pub fn get_contract_size(target: ContractID) -> u64 {
    asm(r1, r2: target) {
        csiz r1 r2;
        r1: u64
    }
}
