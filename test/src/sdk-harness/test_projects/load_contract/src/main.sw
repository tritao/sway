contract;

use std::{load_code::load_external_contract, contract_id::ContractId};

abi Proxy {
    fn set_implementation(target: ContractId);
}

impl Proxy for Contract {
    fn set_implementation(target: ContractId) {
        load_external_contract(target);
    }
}
