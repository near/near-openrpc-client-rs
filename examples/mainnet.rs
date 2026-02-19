//! Example: Query NEAR mainnet for various information.
//!
//! Run with: cargo run --example mainnet

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

    // 4. Validators
    println!("4. Fetching current validators...");
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

    // 5. Network info
    println!("5. Fetching network info...");
    let network = client.network_info().await?;
    println!("   Active peers: {}", network.active_peers.len());
    println!("   Known producers: {}", network.known_producers.len());

    println!("\n=== Done! ===");
    Ok(())
}
