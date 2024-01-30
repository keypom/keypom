use near_units::{parse_gas, parse_near};
use serde_json::json;
use workspaces::{network::Sandbox, Account, Contract, Worker};

const KEYPOM_WASM_PATH: &str = "./out/keypom.wasm";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initiate environemnt
    let worker = workspaces::sandbox().await?;
    println!("Current working directory: {:?}", std::env::current_dir());

    // deploy contracts
    let keypom_wasm = match std::fs::read(KEYPOM_WASM_PATH) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading wasm file: {}", err);
            // Handle the error appropriately, e.g., return an error or panic with a more informative message.
            std::process::exit(1);
        }
    };
    let keypom_contract = worker.dev_deploy(&keypom_wasm).await?;

    // create accounts
    let owner = worker.root_account().unwrap();
    let alice = owner
        .create_subaccount("alice")
        .initial_balance(parse_near!("30 N"))
        .transact()
        .await?
        .into_result()?;

    // Initialize contracts
    keypom_contract
        .call("new")
        .args_json(json!({
            "root_account": owner.id(),
            "owner_id": owner.id(),
            "contract_metadata": {
                "version": "0.1.0",
                "link": "foo"
            }
        }))
        .transact()
        .await?;

    // begin tests
    test_simple(&owner, &alice, &keypom_contract).await?;
    Ok(())
}

async fn test_simple(
    owner: &Account,
    user: &Account,
    keypom_contract: &Contract,
) -> anyhow::Result<()> {
    let total_supply = keypom_contract
        .view("get_key_total_supply")
        .await?
        .json::<u64j>()?;
    println!("total_supply: {:?}", total_supply);
    println!("      Passed ✅ test_simple_approve");
    Ok(())
}
