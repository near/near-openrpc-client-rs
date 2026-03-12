//! RPC error types for the NEAR Protocol JSON-RPC API.
//!
//! This module provides the core [`RpcError`] and [`RpcErrorCause`] types returned by
//! NEAR RPC nodes, as well as per-method typed error enums that can be deserialized
//! from the `cause` field.
//!
//! These types are available without the `client` feature, so downstream crates that
//! only need types can still work with RPC errors.

use serde::{Deserialize, Serialize};

use crate::types::{AccountId, CryptoHash, PublicKey, ShardId};

/// Legacy error response from nearcore's backward-compatible query handling.
///
/// Nearcore returns `UnknownAccessKey` and `ContractExecutionError` as fake success
/// responses (inside the `"result"` field) instead of proper JSON-RPC errors, for
/// backward compatibility. This type captures that legacy shape so callers get a
/// meaningful error instead of a confusing deserialization failure.
///
/// See: <https://github.com/near/nearcore/blob/master/chain/jsonrpc/src/lib.rs>
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{error}")]
pub struct LegacyQueryError {
    /// The error message, e.g. "access key ed25519:... does not exist while viewing"
    pub error: String,
    /// Logs from contract execution (empty for access key errors)
    #[serde(default)]
    pub logs: Vec<String>,
    /// Block height at which the query was executed
    #[serde(default)]
    pub block_height: Option<u64>,
    /// Block hash at which the query was executed
    #[serde(default)]
    pub block_hash: Option<String>,
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
/// deserialized into the appropriate error type (e.g., [`RpcBlockError`])
/// via [`try_cause_as`](RpcError::try_cause_as).
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Returns `true` if this error is likely transient and worth retrying.
    ///
    /// Covers node-syncing states, shard unavailability, timeouts, internal
    /// errors, and request-routing scenarios.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.cause_name(),
            Some("NO_SYNCED_BLOCKS")
                | Some("UNAVAILABLE_SHARD")
                | Some("NOT_SYNCED_YET")
                | Some("TIMEOUT_ERROR")
                | Some("REQUEST_ROUTED")
                | Some("UNKNOWN_EPOCH")
                | Some("VALIDATOR_INFO_UNAVAILABLE")
                | Some("INTERNAL_ERROR")
        )
    }

    /// Attempts to deserialize the error cause into a typed per-method error enum.
    ///
    /// Reconstructs `{"name": cause.name, "info": cause.info}` and deserializes
    /// into `T`. Returns `None` if there is no cause, or `Some(Err(...))` if
    /// deserialization fails (e.g., wrong error type for this method).
    ///
    /// # Example
    ///
    /// ```
    /// # use near_openrpc_client::errors::*;
    /// # let error: RpcError = serde_json::from_str(r#"{
    /// #   "code": -32000, "message": "Server error",
    /// #   "name": "HANDLER_ERROR",
    /// #   "cause": { "name": "UNKNOWN_BLOCK", "info": {} }
    /// # }"#).unwrap();
    /// if let Some(Ok(block_err)) = error.try_cause_as::<RpcBlockError>() {
    ///     match block_err {
    ///         RpcBlockError::UnknownBlock { .. } => { /* handle */ }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn try_cause_as<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Option<Result<T, serde_json::Error>> {
        let cause = self.cause.as_ref()?;
        let value = serde_json::json!({
            "name": cause.name,
            "info": cause.info,
        });
        Some(serde_json::from_value(value))
    }
}

// ── Per-method RPC error enums ──────────────────────────────────────────

/// Errors returned by `query` RPC methods (view_account, view_code, call_function, etc).
///
/// Corresponds to nearcore's `RpcQueryError` in `chain/jsonrpc-primitives`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "name", content = "info", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RpcQueryError {
    #[error("no synced blocks")]
    NoSyncedBlocks,
    #[error("shard {shard_id} unavailable")]
    UnavailableShard { shard_id: ShardId },
    #[error("block has been garbage collected: {block_info}")]
    GarbageCollectedBlock { block_info: serde_json::Value },
    #[error("unknown block: {block_info}")]
    UnknownBlock { block_info: serde_json::Value },
    #[error("invalid account ID: {requested_account_id}")]
    InvalidAccount { requested_account_id: AccountId },
    #[error("unknown account: {requested_account_id} at block {block_height} ({block_hash})")]
    UnknownAccount {
        requested_account_id: AccountId,
        block_height: u64,
        block_hash: CryptoHash,
    },
    #[error(
        "no contract code for account: {contract_account_id} at block {block_height} ({block_hash})"
    )]
    NoContractCode {
        contract_account_id: AccountId,
        block_height: u64,
        block_hash: CryptoHash,
    },
    #[error("contract state too large for account: {contract_account_id}")]
    TooLargeContractState { contract_account_id: AccountId },
    #[error(
        "unknown access key for {public_key} on {requested_account_id} at block {block_height} ({block_hash})"
    )]
    UnknownAccessKey {
        public_key: PublicKey,
        requested_account_id: AccountId,
        block_height: u64,
        block_hash: CryptoHash,
    },
    #[error("unknown gas key")]
    UnknownGasKey,
    #[error("contract execution error: {error_message}")]
    ContractExecutionError {
        error_message: String,
        block_height: u64,
        block_hash: CryptoHash,
    },
    #[error("no global contract code")]
    NoGlobalContractCode,
    #[error("internal error")]
    InternalError { error_message: String },
}

/// Errors returned by the `block` RPC method.
///
/// Corresponds to nearcore's `RpcBlockError` in `chain/jsonrpc-primitives`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "name", content = "info", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RpcBlockError {
    #[error("unknown block: {block_info}")]
    UnknownBlock { block_info: serde_json::Value },
    #[error("node not synced yet")]
    NotSyncedYet,
    #[error("internal error")]
    InternalError { error_message: String },
}

/// Errors returned by transaction RPC methods (send_tx, broadcast_tx_commit, tx).
///
/// Corresponds to nearcore's `RpcTransactionError` in `chain/jsonrpc-primitives`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "name", content = "info", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RpcTransactionError {
    #[error("invalid transaction: {error_message}")]
    InvalidTransaction {
        /// Serialized `InvalidTxError` from nearcore. Use `serde_json::Value`
        /// to stay forward-compatible with new variants.
        error_message: serde_json::Value,
    },
    #[error("node does not track shard {shard_id}")]
    DoesNotTrackShard { shard_id: ShardId },
    #[error("request routed")]
    RequestRouted { transaction_hash: CryptoHash },
    #[error("unknown transaction: {transaction_hash}")]
    UnknownTransaction { transaction_hash: CryptoHash },
    #[error("internal error")]
    InternalError { error_message: String },
    #[error("timeout error")]
    TimeoutError,
}

/// Errors returned by the `validators` RPC method.
///
/// Corresponds to nearcore's `RpcValidatorError` in `chain/jsonrpc-primitives`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "name", content = "info", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RpcValidatorError {
    #[error("unknown epoch")]
    UnknownEpoch,
    #[error("validator info unavailable")]
    ValidatorInfoUnavailable,
    #[error("internal error")]
    InternalError { error_message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_error_round_trip() {
        let json = serde_json::json!({
            "code": -32000,
            "message": "Server error",
            "data": "Block not found",
            "name": "HANDLER_ERROR",
            "cause": {
                "name": "UNKNOWN_BLOCK",
                "info": { "block_info": "abc123" }
            }
        });

        let err: RpcError = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(err.code, -32000);
        assert_eq!(err.message, "Server error");
        assert!(err.is_handler_error());
        assert!(!err.is_internal_error());
        assert!(!err.is_request_validation_error());
        assert_eq!(err.cause_name(), Some("UNKNOWN_BLOCK"));

        // Serialize back and compare
        let roundtrip: serde_json::Value = serde_json::to_value(&err).unwrap();
        assert_eq!(roundtrip["code"], json["code"]);
        assert_eq!(roundtrip["message"], json["message"]);
        assert_eq!(roundtrip["cause"]["name"], json["cause"]["name"]);
    }

    #[test]
    fn try_cause_as_block_error() {
        let json = serde_json::json!({
            "code": -32000,
            "message": "Server error",
            "name": "HANDLER_ERROR",
            "cause": {
                "name": "UNKNOWN_BLOCK",
                "info": { "block_info": "abc123" }
            }
        });

        let err: RpcError = serde_json::from_value(json).unwrap();
        let block_err = err.try_cause_as::<RpcBlockError>().unwrap().unwrap();
        assert!(matches!(block_err, RpcBlockError::UnknownBlock { .. }));
    }

    #[test]
    fn try_cause_as_query_error() {
        let json = serde_json::json!({
            "code": -32000,
            "message": "Server error",
            "name": "HANDLER_ERROR",
            "cause": {
                "name": "UNKNOWN_ACCOUNT",
                "info": {
                    "requested_account_id": "bob.near",
                    "block_height": 12345,
                    "block_hash": "9FMnGHBEfJ3PoKzSaq7EwCotanD3RLGA9UFqEjB3hrN1"
                }
            }
        });

        let err: RpcError = serde_json::from_value(json).unwrap();
        let query_err = err.try_cause_as::<RpcQueryError>().unwrap().unwrap();
        match query_err {
            RpcQueryError::UnknownAccount {
                requested_account_id,
                block_height,
                ..
            } => {
                assert_eq!(requested_account_id.0, "bob.near");
                assert_eq!(block_height, 12345);
            }
            other => panic!("expected UnknownAccount, got: {other:?}"),
        }
    }

    #[test]
    fn try_cause_as_returns_none_without_cause() {
        let json = serde_json::json!({
            "code": -32700,
            "message": "Parse error"
        });

        let err: RpcError = serde_json::from_value(json).unwrap();
        assert!(err.try_cause_as::<RpcBlockError>().is_none());
    }

    #[test]
    fn is_retryable_for_transient_errors() {
        for cause_name in [
            "NO_SYNCED_BLOCKS",
            "UNAVAILABLE_SHARD",
            "NOT_SYNCED_YET",
            "TIMEOUT_ERROR",
            "REQUEST_ROUTED",
            "UNKNOWN_EPOCH",
            "VALIDATOR_INFO_UNAVAILABLE",
            "INTERNAL_ERROR",
        ] {
            let err = RpcError {
                code: -32000,
                message: "Server error".into(),
                data: None,
                name: Some("HANDLER_ERROR".into()),
                cause: Some(RpcErrorCause {
                    name: cause_name.into(),
                    info: None,
                }),
            };
            assert!(err.is_retryable(), "{cause_name} should be retryable");
        }
    }

    #[test]
    fn is_not_retryable_for_permanent_errors() {
        for cause_name in [
            "UNKNOWN_BLOCK",
            "INVALID_ACCOUNT",
            "UNKNOWN_ACCOUNT",
            "UNKNOWN_ACCESS_KEY",
        ] {
            let err = RpcError {
                code: -32000,
                message: "Server error".into(),
                data: None,
                name: Some("HANDLER_ERROR".into()),
                cause: Some(RpcErrorCause {
                    name: cause_name.into(),
                    info: None,
                }),
            };
            assert!(!err.is_retryable(), "{cause_name} should NOT be retryable");
        }
    }

    #[test]
    fn deserialize_transaction_error() {
        let json = serde_json::json!({
            "name": "TIMEOUT_ERROR",
            "info": null
        });

        let err: RpcTransactionError = serde_json::from_value(json).unwrap();
        assert!(matches!(err, RpcTransactionError::TimeoutError));
    }

    #[test]
    fn deserialize_validator_error() {
        let json = serde_json::json!({
            "name": "UNKNOWN_EPOCH",
            "info": null
        });

        let err: RpcValidatorError = serde_json::from_value(json).unwrap();
        assert!(matches!(err, RpcValidatorError::UnknownEpoch));
    }

    #[test]
    fn deserialize_query_error_no_synced_blocks() {
        let json = serde_json::json!({
            "name": "NO_SYNCED_BLOCKS",
            "info": null
        });

        let err: RpcQueryError = serde_json::from_value(json).unwrap();
        assert!(matches!(err, RpcQueryError::NoSyncedBlocks));
    }

    #[test]
    fn deserialize_query_error_contract_execution() {
        let json = serde_json::json!({
            "name": "CONTRACT_EXECUTION_ERROR",
            "info": {
                "error_message": "wasm execution failed",
                "block_height": 99999,
                "block_hash": "4reLvkAWfqk5fsqio1KLudk46cqRz9erQdaHkWZKMJDZ"
            }
        });

        let err: RpcQueryError = serde_json::from_value(json).unwrap();
        match err {
            RpcQueryError::ContractExecutionError {
                error_message,
                block_height,
                ..
            } => {
                assert_eq!(error_message, "wasm execution failed");
                assert_eq!(block_height, 99999);
            }
            other => panic!("expected ContractExecutionError, got: {other:?}"),
        }
    }

    #[test]
    fn deserialize_legacy_query_error() {
        let json = serde_json::json!({
            "error": "access key ed25519:abc does not exist while viewing",
            "logs": [],
            "block_height": 100,
            "block_hash": "9FMnGHBEfJ3PoKzSaq7EwCotanD3RLGA9UFqEjB3hrN1"
        });

        let err: super::LegacyQueryError = serde_json::from_value(json).unwrap();
        assert!(err.error.contains("does not exist"));
        assert_eq!(err.block_height, Some(100));
        assert!(err.logs.is_empty());
    }

    #[test]
    fn deserialize_transaction_invalid() {
        let json = serde_json::json!({
            "name": "INVALID_TRANSACTION",
            "info": {
                "error_message": {"kind": "InvalidNonce", "nonce": 5, "ak_nonce": 10}
            }
        });

        let err: RpcTransactionError = serde_json::from_value(json).unwrap();
        match err {
            RpcTransactionError::InvalidTransaction { error_message } => {
                assert!(error_message.is_object());
            }
            other => panic!("expected InvalidTransaction, got: {other:?}"),
        }
    }
}
