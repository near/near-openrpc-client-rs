//! Generated Rust types and async client for the NEAR Protocol JSON-RPC API.
//!
//! This crate provides strongly-typed request/response types and an ergonomic async
//! client, all generated directly from NEAR's official
//! [OpenRPC specification](https://github.com/near/nearcore/blob/master/chain/jsonrpc/openapi/openrpc.json).
//!
//! # Features
//!
//! - **`types` module** — All RPC request/response types, generated at build time via
//!   [`typify`](https://docs.rs/typify). Available with no additional features.
//! - **`client` module** (enabled by default) — An async RPC client built on `reqwest`.
//!
//! # Quick start
//!
//! ```no_run
//! use near_openrpc_client::{NearRpcClient, types::*};
//!
//! #[tokio::main]
//! async fn main() -> near_openrpc_client::client::Result<()> {
//!     let client = NearRpcClient::mainnet();
//!     let status = client.status().await?;
//!     println!("Chain: {} at block {}", status.chain_id, status.sync_info.latest_block_height);
//!     Ok(())
//! }
//! ```

mod query_helpers;
mod token_helpers;
pub mod types;

pub use query_helpers::BlockReference;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "client")]
pub use client::NearRpcClient;

pub use types::*;
