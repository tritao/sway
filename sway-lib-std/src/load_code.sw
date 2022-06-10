//! Functionality for loading code from external contracts
library load_code;

use ::contract_id::ContractId;
use ::logging::log;
use ::context::registers::*;
use ::assert::*;
use ::revert::revert;

/// This function can be used to concatenate the code of multiple contracts
/// together by appending the bytecode of `target` contract to that of the
/// current contract. It can only be used when the stack area of the call frame /// is unused (i.e. prior to being used).
/// You may also use `stash_stack()` and `restore_stack` to manually ensure
/// an empty stack by temporarily moving stack values to the heap, ie:
///
///     ```
///     stash_stack();
///     load_contract(some_target_contract);
///     restore_stack();
///     ```

pub fn load_contract(target: ContractId) {
    asm(id: target, offset: 0, r3: contract_size(target), r4) {
        ldc id offset r3;
    };
}

/// Get the size in bytes of the contract 'target'
pub fn contract_size(target: ContractId) -> u64 {
    asm(r1, r2: target) {
        csiz r1 r2;
        r1: u64
    }
}


// Move values on the stack to the heap
fn stash_stack() {
    let stack_size: u64 = stack_ptr() - stack_start_ptr();
    // store stack length at current bottom of heap
    let size: [u8; 1] = [stack_size];

    asm(r1, r2, size: size, bytes: 8) {
        aloc size;        // allocate `size` bytes on the heap
        addi r1 hp i1;    // compensate for off-by-1 $hp initialization
        mcp r1 ssp size;  // copy `size` bytes from the stack to the heap
        aloc bytes;       // allocate a single word on the heap
        sw r2 size i0;    // store the value of size at the end of the heap
        mcl ssp size;     // clear the stack
    }
}

fn restore_stack(size: u64) {
    asm(r0, size: size) {
        // $sp should already == $ssp, no need to move sp
        cfe size; // allocate 16 bytes on the stack; need this to be dynamic !
        // alternately, allocate max stack space, then deallocate unused after mcp
    }
}
