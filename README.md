# near-openrpc-client

Generated Rust types and async client for the [NEAR Protocol](https://near.org) JSON-RPC API, built directly from NEAR's official [OpenRPC specification](https://github.com/near/nearcore/blob/master/chain/jsonrpc/openapi/openrpc.json).

## Quick start

```rust
use near_openrpc_client::{NearRpcClient, types::*};

#[tokio::main]
async fn main() -> near_openrpc_client::client::Result<()> {
    let client = NearRpcClient::mainnet();

    // Get node status
    let status = client.status().await?;
    println!("Chain: {} at block {}", status.chain_id, status.sync_info.latest_block_height);

    // Query an account
    let account = client.query(RpcQueryRequest::ViewAccountFinality {
        finality: Finality::Final,
        account_id: "near".parse().unwrap(),
        request_type: "view_account".to_string(),
    }).await?;

    Ok(())
}
```

## Features

- **200+ strongly-typed structs** generated from the OpenRPC schema via [`typify`](https://docs.rs/typify)
- **Async client** with convenience constructors for mainnet/testnet/betanet/local
- **Types-only mode** — disable the `client` feature to use just the types with no `reqwest`/`tokio` dependency

```toml
# Full client (default)
near-openrpc-client = "0.1"

# Types only
near-openrpc-client = { version = "0.1", default-features = false }
```

## How it works

At build time, `build.rs` reads `openrpc.json`, transforms the schema (including a cartesian-product expansion of `allOf` refs for better enum variant names), and feeds it to `typify` to generate `src/generated.rs`.

A daily GitHub Action fetches the latest spec from nearcore and opens a PR if anything changed.

## Running the example

```sh
cargo run --example mainnet
```

## License

MIT OR Apache-2.0
