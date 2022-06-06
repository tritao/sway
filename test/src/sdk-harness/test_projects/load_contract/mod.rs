use fuels::{prelude::*, tx::ContractId, };
use fuels_abigen_macro::abigen;
use fuels::signers::wallet::Wallet;

// Load abi from json
abigen!(Proxy, "./test_projects/load_contract/out/debug/load_contract-abi.json");

abigen!(ProxyImplementation, "./test_artifacts/proxy_implementation_contract/out/debug/proxy_implementation_contract-abi.json");

async fn get_contract_instance() -> (Proxy, ContractId, ContractId, Wallet) {
    // Launch a local network and deploy the contract
    let wallet = launch_provider_and_get_single_wallet().await;

    let proxy_id = Contract::deploy("test_projects/load_contract/out/debug/load_contract.bin", &wallet, TxParameters::default())
        .await
        .unwrap();

    let implementation_id = Contract::deploy("test_artifacts/proxy_implementation_contract/out/debug/proxy_implementation_contract.bin", &wallet, TxParameters::default())
        .await
        .unwrap();

    let instance = Proxy::new(proxy_id.to_string(), wallet.clone());

    (instance, proxy_id, implementation_id, wallet)
}

#[tokio::test]
async fn can_call_implementation_via_proxy() {
    let (instance, proxy_id, implementation_id, wallet) = get_contract_instance().await;

    // call set_implementation to load the bytecode for the proxy implementation contract.
    instance.set_implementation(implementation_id)
        .set_contracts(&[implementation_id])
        .call()
        .await
        .unwrap();

    // now we need to connect to the existing Proxy contract, but using the abi for the Implementation contract
    let implementation_aware_proxy_instance = ProxyImplementation::new(proxy_id.to_string(), wallet);

    // we should now be able to access functions on the implementation contract
    // through the proxy contract
    let response = implementation_aware_proxy_instance.get_42()
        .simulate()
        .await
        .unwrap();

    assert_eq!(response.value, 42);

}

// #[tokio::test]
// async fn can_modify_proxy_storage() {}
