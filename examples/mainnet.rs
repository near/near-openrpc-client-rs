//! Example: Query NEAR mainnet for various information.
//!
//! Run with: cargo run --example mainnet

use base64::{Engine, engine::general_purpose::STANDARD};
use near_openrpc_client::{NearRpcClient, client::Result, types::*};

#[tokio::main]
async fn main() -> Result<()> {
    let client = NearRpcClient::mainnet();

    println!("=== NEAR Mainnet RPC Client ===\n");

    // 1. Node status
    println!("1. Fetching node status...");
    let status = client.status().await?;
    println!("   Chain ID: {}", status.chain_id);
    println!("   Protocol version: {}", status.protocol_version);
    println!(
        "   Latest block height: {}",
        status.sync_info.latest_block_height
    );
    println!(
        "   Latest block hash: {}",
        status.sync_info.latest_block_hash
    );
    println!("   Syncing: {}", status.sync_info.syncing);
    println!();

    // 2. Latest finalized block
    println!("2. Fetching latest block...");
    let block = client
        .block(RpcBlockRequest::Finality(Finality::Final))
        .await?;
    println!("   Block height: {}", block.header.height);
    println!("   Block hash: {}", block.header.hash);
    println!("   Timestamp: {}", block.header.timestamp);
    println!("   Author: {}", block.author);
    println!("   Chunks: {}", block.chunks.len());
    println!();

    // 3. Gas price
    println!("3. Fetching gas price...");
    let gas = client
        .gas_price(RpcGasPriceRequest { block_id: None })
        .await?;
    println!("   Gas price: {} yoctoNEAR", gas.gas_price);
    println!();

    // 4. View account
    println!("4. Viewing account 'near'...");
    let account = client
        .view_account(RpcViewAccountRequest::FinalityAccountId {
            account_id: "near".parse().unwrap(),
            finality: Finality::Final,
        })
        .await?;
    println!("   Balance: {} yoctoNEAR", account.amount);
    println!("   Locked: {} yoctoNEAR", account.locked);
    println!("   Storage usage: {} bytes", account.storage_usage);
    println!("   Code hash: {}", account.code_hash);
    println!();

    // 5. Call view function
    println!("5. Calling view function on wrap.near...");
    let args = serde_json::json!({"account_id": "near"});
    let result = client
        .call_function(RpcCallFunctionRequest::FinalityAccountId {
            account_id: "wrap.near".parse().unwrap(),
            method_name: "ft_balance_of".to_string(),
            args_base64: STANDARD.encode(args.to_string()).into(),
            finality: Finality::Final,
        })
        .await?;
    println!("   Result: {}", String::from_utf8_lossy(&result.result));
    println!("   Block height: {}", result.block_height);
    println!();

    // 6. View access keys
    println!("6. Viewing access keys for 'near'...");
    let keys = client
        .view_access_key_list(RpcViewAccessKeyListRequest::FinalityAccountId {
            account_id: "near".parse().unwrap(),
            finality: Finality::Final,
        })
        .await?;
    println!("   Access keys: {}", keys.keys.len());
    println!();

    // 7. Validators
    println!("7. Fetching current validators...");
    let validators = client.validators(RpcValidatorRequest::Latest).await?;
    println!(
        "   Current validators: {}",
        validators.current_validators.len()
    );
    println!("   Next validators: {}", validators.next_validators.len());
    if let Some(first) = validators.current_validators.first() {
        println!(
            "   Top validator: {} (stake: {})",
            first.account_id, first.stake
        );
    }
    println!();

    // 8. Network info
    println!("8. Fetching network info...");
    let network = client.network_info().await?;
    println!("   Active peers: {}", network.active_peers.len());
    println!("   Known producers: {}", network.known_producers.len());

    println!("\n=== Done! ===");
    Ok(())
}
