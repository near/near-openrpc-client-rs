//! Example: Query NEAR mainnet for various information.
//!
//! Run with: cargo run --example mainnet

use near_openrpc_client::{client::Result, types::*, NearRpcClient};

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

    // 4. Query account
    println!("4. Querying 'near' foundation account...");
    let account_query = client
        .query(RpcQueryRequest::ViewAccountFinality {
            finality: Finality::Final,
            account_id: "near".parse().expect("valid account id"),
            request_type: "view_account".to_string(),
        })
        .await?;

    if let RpcQueryResponse::AccountView {
        amount,
        storage_usage,
        ..
    } = account_query
    {
        let balance_str: String = amount.into();
        let balance_yocto: u128 = balance_str.parse().unwrap_or(0);
        let balance_near = balance_yocto / 10u128.pow(24);
        println!("   Account: near");
        println!("   Balance: {} NEAR", balance_near);
        println!("   Storage used: {} bytes", storage_usage);
    }
    println!();

    // 5. Call a view function
    println!("5. Calling ft_metadata on wrap.near...");
    let call_result = client
        .query(RpcQueryRequest::CallFunctionFinality {
            finality: Finality::Final,
            account_id: "wrap.near".parse().expect("valid account id"),
            method_name: "ft_metadata".to_string(),
            args_base64: "e30=".parse().expect("valid base64"), // "{}"
            request_type: "call_function".to_string(),
        })
        .await?;

    if let RpcQueryResponse::CallResult { result, .. } = call_result {
        let decoded = String::from_utf8(result).unwrap_or_default();
        println!("   wNEAR metadata: {}", decoded);
    }
    println!();

    // 6. Validators
    println!("6. Fetching current validators...");
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

    // 7. Network info
    println!("7. Fetching network info...");
    let network = client.network_info().await?;
    println!("   Active peers: {}", network.active_peers.len());
    println!("   Known producers: {}", network.known_producers.len());

    println!("\n=== Done! ===");
    Ok(())
}
