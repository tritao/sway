//! Functionality for loading code from external contracts
library load_code;

use ::contract_id::ContractId;

/// Load an external contract `target`. The bytecode will be appended to the current contract's bytecode, allowing usage of the new functionality via jumps rather than calls.
pub fn load_external_contract(target: ContractId) {
    let BYTES: u64 = get_contract_size(target);
    asm(id: target, bytecode_ptr: 0, r3: BYTES) {
        ldc id bytecode_ptr r3;
    };
}

/// Get the size in bytes of the contract 'target'
pub fn get_contract_size(target: ContractId) -> u64 {
    asm(r1, r2: target) {
        csiz r1 r2;
        r1: u64
    }
}
