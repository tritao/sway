//! Functionality for loading code from external contracts
library load_code;

use ::contract_id::ContractId;
use ::logging::log;
use ::context::registers::*;
use ::assert::*;
use ::revert::revert;

/// Load an external contract `target`. The bytecode will be appended to the current contract's bytecode, allowing usage of the new functionality via jumps rather than calls.
pub fn load_external_contract(target: ContractId) {

    // let vm_max_ram = 67_108_864;
    // let contract_max_size = 16_777_216;

    if stack_start_ptr() + get_contract_size(target) > 67_108_864 {
        revert(1);
    } else if stack_start_ptr() + get_contract_size(target) > 67_108_864 {
        revert(2);
    } else if stack_ptr() != stack_start_ptr() {
        revert(3);
    } else if stack_start_ptr() + get_contract_size(target) > heap_ptr() {
        revert(4);
    } else if get_contract_size(target) > 16_777_216 {
        revert(5);
    } else if get_contract_size(target) > 67_108_864 {
        revert(6);
    };

    asm(id: target, offset: 0, r3: get_contract_size(target), r4) {
        // move r4 sp;
        // cfei i8;
        // sw r4 id i0;
        ldc id offset r3;
    };
}

/// Get the size in bytes of the contract 'target'
pub fn get_contract_size(target: ContractId) -> u64 {
    asm(r1, r2: target) {
        csiz r1 r2;
        r1: u64
    }
}
