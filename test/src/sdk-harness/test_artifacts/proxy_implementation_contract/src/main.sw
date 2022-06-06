contract;

use proxy_implementation_abi::ProxyImplementation;

impl ProxyImplementation for Contract {
    fn get_42() -> u64 {
        42
    }
}
