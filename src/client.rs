//! Async JSON-RPC client for NEAR Protocol.

use crate::types::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// JSON-RPC request wrapper
#[derive(Debug, Serialize)]
struct RpcRequest<T> {
    jsonrpc: &'static str,
    id: u64,
    method: &'static str,
    params: T,
}

/// JSON-RPC response wrapper
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    #[serde(flatten)]
    result: RpcResult<T>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RpcResult<T> {
    Ok { result: T },
    Err { error: RpcError },
}

/// JSON-RPC error returned by the NEAR node.
///
/// NEAR's RPC extends the standard JSON-RPC error with `name` and `cause` fields
/// that carry structured, typed error information. The `data` field is deprecated
/// in nearcore and typically contains only a human-readable string.
///
/// # Error structure
///
/// For handler errors (most RPC failures), the response looks like:
///
/// ```json
/// {
///   "code": -32000,
///   "message": "Server error",
///   "data": "...",
///   "name": "HANDLER_ERROR",
///   "cause": { "name": "UNKNOWN_BLOCK", "info": { ... } }
/// }
/// ```
///
/// The `cause` field contains the per-method typed error, which can be
/// deserialized into the appropriate error type (e.g., `RpcBlockError`).
#[derive(Debug, Deserialize, thiserror::Error)]
#[error("RPC error {code}: {message}")]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    /// Deprecated by nearcore. Prefer `cause` for structured error data.
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// Error category: `HANDLER_ERROR`, `REQUEST_VALIDATION_ERROR`, or `INTERNAL_ERROR`.
    #[serde(default)]
    pub name: Option<String>,
    /// Structured error detail. For handler errors, contains `{"name": "...", "info": {...}}`.
    #[serde(default)]
    pub cause: Option<RpcErrorCause>,
}

/// Structured cause of an RPC error.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcErrorCause {
    /// The error variant name (e.g., `UNKNOWN_BLOCK`, `INVALID_ACCOUNT`).
    pub name: String,
    /// Additional structured information about the error.
    #[serde(default)]
    pub info: Option<serde_json::Value>,
}

impl RpcError {
    /// Returns `true` if this is a handler error (a method-specific failure).
    pub fn is_handler_error(&self) -> bool {
        self.name.as_deref() == Some("HANDLER_ERROR")
    }

    /// Returns `true` if this is a request validation error.
    pub fn is_request_validation_error(&self) -> bool {
        self.name.as_deref() == Some("REQUEST_VALIDATION_ERROR")
    }

    /// Returns `true` if this is an internal error (timeout, connection closed, etc).
    pub fn is_internal_error(&self) -> bool {
        self.name.as_deref() == Some("INTERNAL_ERROR")
    }

    /// Returns the error cause name if available (e.g., `"UNKNOWN_BLOCK"`).
    pub fn cause_name(&self) -> Option<&str> {
        self.cause.as_ref().map(|c| c.name.as_str())
    }
}

/// Client error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type alias for client operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Async client for the NEAR Protocol JSON-RPC API.
///
/// # Example
///
/// ```no_run
/// use near_openrpc_client::NearRpcClient;
///
/// #[tokio::main]
/// async fn main() -> near_openrpc_client::client::Result<()> {
///     let client = NearRpcClient::mainnet();
///     let status = client.status().await?;
///     println!("Chain ID: {}", status.chain_id);
///     Ok(())
/// }
/// ```
pub struct NearRpcClient {
    client: Client,
    url: String,
    request_id: AtomicU64,
}

impl NearRpcClient {
    /// Create a new client with a custom URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            url: url.into(),
            request_id: AtomicU64::new(1),
        }
    }

    /// Create a client for NEAR Mainnet.
    pub fn mainnet() -> Self {
        Self::new("https://rpc.mainnet.near.org")
    }

    /// Create a client for NEAR Testnet.
    pub fn testnet() -> Self {
        Self::new("https://rpc.testnet.near.org")
    }

    /// Create a client for NEAR Betanet.
    pub fn betanet() -> Self {
        Self::new("https://rpc.betanet.near.org")
    }

    /// Create a client for local development (localhost:3030).
    pub fn local() -> Self {
        Self::new("http://localhost:3030")
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &'static str,
        params: P,
    ) -> Result<R> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: self.next_id(),
            method,
            params,
        };

        let response: RpcResponse<R> = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        match response.result {
            RpcResult::Ok { result } => Ok(result),
            RpcResult::Err { error } => Err(Error::Rpc(error)),
        }
    }

    // ── Core ─────────────────────────────────────────────────────

    /// Returns the current status of the node.
    pub async fn status(&self) -> Result<RpcStatusResponse> {
        self.call("status", serde_json::json!({})).await
    }

    /// Returns health status of the node.
    pub async fn health(&self) -> Result<RpcHealthResponse> {
        self.call("health", serde_json::json!({})).await
    }

    /// Queries the current state of node network connections.
    pub async fn network_info(&self) -> Result<RpcNetworkInfoResponse> {
        self.call("network_info", serde_json::json!({})).await
    }

    // ── Block / Chunk ────────────────────────────────────────────

    /// Returns block details for a given height or hash.
    pub async fn block(&self, request: RpcBlockRequest) -> Result<RpcBlockResponse> {
        self.call("block", request).await
    }

    /// Returns details of a specific chunk.
    pub async fn chunk(&self, request: RpcChunkRequest) -> Result<RpcChunkResponse> {
        self.call("chunk", request).await
    }

    /// Returns gas price for a specific block.
    pub async fn gas_price(&self, request: RpcGasPriceRequest) -> Result<RpcGasPriceResponse> {
        self.call("gas_price", request).await
    }

    // ── Query ────────────────────────────────────────────────────

    /// Returns account information for a given account ID.
    pub async fn view_account(
        &self,
        request: RpcViewAccountRequest,
    ) -> Result<RpcViewAccountResponse> {
        self.call("EXPERIMENTAL_view_account", request).await
    }

    /// Returns the contract code (Wasm binary) deployed to an account.
    pub async fn view_code(&self, request: RpcViewCodeRequest) -> Result<RpcViewCodeResponse> {
        self.call("EXPERIMENTAL_view_code", request).await
    }

    /// Returns contract state (key-value pairs) based on a key prefix.
    pub async fn view_state(&self, request: RpcViewStateRequest) -> Result<RpcViewStateResponse> {
        self.call("EXPERIMENTAL_view_state", request).await
    }

    /// Returns information about a single access key for an account.
    pub async fn view_access_key(
        &self,
        request: RpcViewAccessKeyRequest,
    ) -> Result<RpcViewAccessKeyResponse> {
        self.call("EXPERIMENTAL_view_access_key", request).await
    }

    /// Returns all access keys for an account.
    pub async fn view_access_key_list(
        &self,
        request: RpcViewAccessKeyListRequest,
    ) -> Result<RpcViewAccessKeyListResponse> {
        self.call("EXPERIMENTAL_view_access_key_list", request)
            .await
    }

    /// Calls a view function on a contract and returns the result.
    pub async fn call_function(
        &self,
        request: RpcCallFunctionRequest,
    ) -> Result<RpcCallFunctionResponse> {
        self.call("EXPERIMENTAL_call_function", request).await
    }

    // ── Transactions ─────────────────────────────────────────────

    /// Sends a signed transaction asynchronously. Returns immediately with the transaction hash.
    pub async fn broadcast_tx_async(
        &self,
        request: RpcSendTransactionRequest,
    ) -> Result<CryptoHash> {
        self.call("broadcast_tx_async", request).await
    }

    /// Sends a signed transaction and waits for it to complete.
    pub async fn broadcast_tx_commit(
        &self,
        request: RpcSendTransactionRequest,
    ) -> Result<RpcTransactionResponse> {
        self.call("broadcast_tx_commit", request).await
    }

    /// Sends a signed transaction (alias for broadcast_tx_commit).
    pub async fn send_tx(
        &self,
        request: RpcSendTransactionRequest,
    ) -> Result<RpcTransactionResponse> {
        self.call("send_tx", request).await
    }

    /// Queries status of a transaction by hash.
    pub async fn tx(&self, request: RpcTransactionStatusRequest) -> Result<RpcTransactionResponse> {
        self.call("tx", request).await
    }

    // ── Validators ───────────────────────────────────────────────

    /// Queries active validators on the network for a given epoch.
    pub async fn validators(&self, request: RpcValidatorRequest) -> Result<RpcValidatorResponse> {
        self.call("validators", request).await
    }

    /// Returns validators ordered by stake for a given epoch.
    pub async fn validators_ordered(
        &self,
        request: RpcValidatorsOrderedRequest,
    ) -> Result<ValidatorStakeViews> {
        self.call("EXPERIMENTAL_validators_ordered", request).await
    }

    // ── Light Client ─────────────────────────────────────────────

    /// Returns execution proof for light clients (transaction or receipt).
    pub async fn light_client_proof(
        &self,
        request: RpcLightClientExecutionProofRequest,
    ) -> Result<RpcLightClientExecutionProofResponse> {
        self.call("light_client_proof", request).await
    }

    /// Returns the next light client block.
    pub async fn next_light_client_block(
        &self,
        request: RpcLightClientNextBlockRequest,
    ) -> Result<RpcLightClientNextBlockResponse> {
        self.call("next_light_client_block", request).await
    }

    /// Returns block proof for light clients.
    pub async fn light_client_block_proof(
        &self,
        request: RpcLightClientBlockProofRequest,
    ) -> Result<RpcLightClientBlockProofResponse> {
        self.call("light_client_block_proof", request).await
    }

    // ── State Changes ────────────────────────────────────────────

    /// Returns changes in block for a given block height or hash.
    pub async fn changes_in_block(
        &self,
        request: RpcStateChangesInBlockRequest,
    ) -> Result<RpcStateChangesInBlockByTypeResponse> {
        self.call("EXPERIMENTAL_changes_in_block", request).await
    }

    /// Returns state changes for specific state change kinds.
    pub async fn changes(
        &self,
        request: RpcStateChangesInBlockByTypeRequest,
    ) -> Result<RpcStateChangesInBlockResponse> {
        self.call("EXPERIMENTAL_changes", request).await
    }

    /// Returns changes in block (alias for changes_in_block).
    pub async fn block_effects(
        &self,
        request: RpcStateChangesInBlockRequest,
    ) -> Result<RpcStateChangesInBlockByTypeResponse> {
        self.call("block_effects", request).await
    }

    // ── Config ───────────────────────────────────────────────────

    /// Returns protocol configuration for a given block.
    pub async fn protocol_config(
        &self,
        request: RpcProtocolConfigRequest,
    ) -> Result<RpcProtocolConfigResponse> {
        self.call("EXPERIMENTAL_protocol_config", request).await
    }

    /// Returns genesis configuration of the network.
    pub async fn genesis_config(&self) -> Result<GenesisConfig> {
        self.call("genesis_config", serde_json::json!({})).await
    }

    /// Returns client configuration.
    pub async fn client_config(&self) -> Result<RpcClientConfigResponse> {
        self.call("client_config", serde_json::json!({})).await
    }

    // ── Other ────────────────────────────────────────────────────

    /// Returns a receipt by receipt_id.
    pub async fn receipt(&self, request: RpcReceiptRequest) -> Result<RpcReceiptResponse> {
        self.call("EXPERIMENTAL_receipt", request).await
    }

    /// Returns maintenance windows.
    pub async fn maintenance_windows(&self) -> Result<BlockHeightRanges> {
        self.call("maintenance_windows", serde_json::json!({}))
            .await
    }

    /// Returns split storage info.
    pub async fn split_storage_info(&self) -> Result<RpcSplitStorageInfoResponse> {
        self.call("EXPERIMENTAL_split_storage_info", serde_json::json!({}))
            .await
    }

    /// Returns congestion level for a chunk.
    pub async fn congestion_level(
        &self,
        request: RpcCongestionLevelRequest,
    ) -> Result<RpcCongestionLevelResponse> {
        self.call("EXPERIMENTAL_congestion_level", request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = NearRpcClient::mainnet();
        assert_eq!(client.url, "https://rpc.mainnet.near.org");

        let client = NearRpcClient::testnet();
        assert_eq!(client.url, "https://rpc.testnet.near.org");

        let client = NearRpcClient::new("https://custom.rpc.near.org");
        assert_eq!(client.url, "https://custom.rpc.near.org");
    }
}
