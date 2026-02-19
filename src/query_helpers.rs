//! Ergonomic constructors for [`RpcQueryRequest`] that automatically set the
//! `request_type` field to the correct value.

use crate::types::{
    AccountId, BlockId, CryptoHash, Finality, FunctionArgs, PublicKey, RpcQueryRequest, StoreKey,
    SyncCheckpoint,
};

/// How to reference a specific block for a query.
#[derive(Clone, Debug)]
pub enum BlockReference {
    /// Use a finality level (Final, Optimistic, NearFinal).
    Finality(Finality),
    /// Use a specific block height or hash.
    BlockId(BlockId),
    /// Use a sync checkpoint (Genesis, EarliestAvailable).
    SyncCheckpoint(SyncCheckpoint),
}

impl From<Finality> for BlockReference {
    fn from(value: Finality) -> Self {
        Self::Finality(value)
    }
}

impl From<BlockId> for BlockReference {
    fn from(value: BlockId) -> Self {
        Self::BlockId(value)
    }
}

impl From<SyncCheckpoint> for BlockReference {
    fn from(value: SyncCheckpoint) -> Self {
        Self::SyncCheckpoint(value)
    }
}

impl RpcQueryRequest {
    /// Construct a `view_account` query with the correct `request_type`.
    pub fn view_account(
        account_id: impl Into<AccountId>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewAccountFinality {
                account_id,
                finality,
                request_type: "view_account".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewAccountBlockId {
                account_id,
                block_id,
                request_type: "view_account".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => Self::ViewAccountSyncCheckpoint {
                account_id,
                sync_checkpoint,
                request_type: "view_account".to_string(),
            },
        }
    }

    /// Construct a `view_code` query with the correct `request_type`.
    pub fn view_code(
        account_id: impl Into<AccountId>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewCodeFinality {
                account_id,
                finality,
                request_type: "view_code".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewCodeBlockId {
                account_id,
                block_id,
                request_type: "view_code".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => Self::ViewCodeSyncCheckpoint {
                account_id,
                sync_checkpoint,
                request_type: "view_code".to_string(),
            },
        }
    }

    /// Construct a `view_state` query with the correct `request_type`.
    ///
    /// Sets `include_proof` to `None` by default. Construct the variant directly
    /// if you need to set `include_proof`.
    pub fn view_state(
        account_id: impl Into<AccountId>,
        prefix_base64: StoreKey,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewStateFinality {
                account_id,
                finality,
                include_proof: None,
                prefix_base64,
                request_type: "view_state".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewStateBlockId {
                account_id,
                block_id,
                include_proof: None,
                prefix_base64,
                request_type: "view_state".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => Self::ViewStateSyncCheckpoint {
                account_id,
                include_proof: None,
                prefix_base64,
                request_type: "view_state".to_string(),
                sync_checkpoint,
            },
        }
    }

    /// Construct a `view_access_key` query with the correct `request_type`.
    pub fn view_access_key(
        account_id: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        let public_key = public_key.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewAccessKeyFinality {
                account_id,
                finality,
                public_key,
                request_type: "view_access_key".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewAccessKeyBlockId {
                account_id,
                block_id,
                public_key,
                request_type: "view_access_key".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => Self::ViewAccessKeySyncCheckpoint {
                account_id,
                public_key,
                request_type: "view_access_key".to_string(),
                sync_checkpoint,
            },
        }
    }

    /// Construct a `view_access_key_list` query with the correct `request_type`.
    pub fn view_access_key_list(
        account_id: impl Into<AccountId>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewAccessKeyListFinality {
                account_id,
                finality,
                request_type: "view_access_key_list".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewAccessKeyListBlockId {
                account_id,
                block_id,
                request_type: "view_access_key_list".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => {
                Self::ViewAccessKeyListSyncCheckpoint {
                    account_id,
                    request_type: "view_access_key_list".to_string(),
                    sync_checkpoint,
                }
            }
        }
    }

    /// Construct a `view_gas_key_nonces` query with the correct `request_type`.
    pub fn view_gas_key_nonces(
        account_id: impl Into<AccountId>,
        public_key: impl Into<PublicKey>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        let public_key = public_key.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewGasKeyNoncesFinality {
                account_id,
                finality,
                public_key,
                request_type: "view_gas_key_nonces".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewGasKeyNoncesBlockId {
                account_id,
                block_id,
                public_key,
                request_type: "view_gas_key_nonces".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => {
                Self::ViewGasKeyNoncesSyncCheckpoint {
                    account_id,
                    public_key,
                    request_type: "view_gas_key_nonces".to_string(),
                    sync_checkpoint,
                }
            }
        }
    }

    /// Construct a `call_function` query with the correct `request_type`.
    pub fn call_function(
        account_id: impl Into<AccountId>,
        method_name: impl Into<String>,
        args_base64: impl Into<FunctionArgs>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        let method_name = method_name.into();
        let args_base64 = args_base64.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::CallFunctionFinality {
                account_id,
                args_base64,
                finality,
                method_name,
                request_type: "call_function".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::CallFunctionBlockId {
                account_id,
                args_base64,
                block_id,
                method_name,
                request_type: "call_function".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => Self::CallFunctionSyncCheckpoint {
                account_id,
                args_base64,
                method_name,
                request_type: "call_function".to_string(),
                sync_checkpoint,
            },
        }
    }

    /// Construct a `view_global_contract_code` query with the correct `request_type`.
    pub fn view_global_contract_code(
        code_hash: impl Into<CryptoHash>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let code_hash = code_hash.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewGlobalContractCodeFinality {
                code_hash,
                finality,
                request_type: "view_global_contract_code".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewGlobalContractCodeBlockId {
                block_id,
                code_hash,
                request_type: "view_global_contract_code".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => {
                Self::ViewGlobalContractCodeSyncCheckpoint {
                    code_hash,
                    request_type: "view_global_contract_code".to_string(),
                    sync_checkpoint,
                }
            }
        }
    }

    /// Construct a `view_global_contract_code_by_account_id` query with the correct
    /// `request_type`.
    pub fn view_global_contract_code_by_account_id(
        account_id: impl Into<AccountId>,
        block_ref: impl Into<BlockReference>,
    ) -> Self {
        let account_id = account_id.into();
        match block_ref.into() {
            BlockReference::Finality(finality) => Self::ViewGlobalContractCodeByAccountIdFinality {
                account_id,
                finality,
                request_type: "view_global_contract_code_by_account_id".to_string(),
            },
            BlockReference::BlockId(block_id) => Self::ViewGlobalContractCodeByAccountIdBlockId {
                account_id,
                block_id,
                request_type: "view_global_contract_code_by_account_id".to_string(),
            },
            BlockReference::SyncCheckpoint(sync_checkpoint) => {
                Self::ViewGlobalContractCodeByAccountIdSyncCheckpoint {
                    account_id,
                    request_type: "view_global_contract_code_by_account_id".to_string(),
                    sync_checkpoint,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // Helper to extract `request_type` from serialized JSON.
    fn request_type_of(req: &RpcQueryRequest) -> String {
        let json: Value = serde_json::to_value(req).expect("serialization should succeed");
        json["request_type"]
            .as_str()
            .expect("request_type should be a string")
            .to_string()
    }

    // ── BlockReference From impls ───────────────────────────────────────

    #[test]
    fn block_reference_from_finality() {
        let br: BlockReference = Finality::Final.into();
        assert!(matches!(br, BlockReference::Finality(Finality::Final)));
    }

    #[test]
    fn block_reference_from_block_id() {
        let br: BlockReference = BlockId::BlockHeight(42).into();
        assert!(matches!(
            br,
            BlockReference::BlockId(BlockId::BlockHeight(42))
        ));
    }

    #[test]
    fn block_reference_from_sync_checkpoint() {
        let br: BlockReference = SyncCheckpoint::Genesis.into();
        assert!(matches!(
            br,
            BlockReference::SyncCheckpoint(SyncCheckpoint::Genesis)
        ));
    }

    // ── Constructor request_type correctness ────────────────────────────

    #[test]
    fn view_account_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_account("near".to_string(), block_ref);
            assert_eq!(request_type_of(&req), "view_account");
        }
    }

    #[test]
    fn view_code_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_code("near".to_string(), block_ref);
            assert_eq!(request_type_of(&req), "view_code");
        }
    }

    #[test]
    fn view_state_sets_request_type() {
        for block_ref in all_block_refs() {
            let req =
                RpcQueryRequest::view_state("near".to_string(), StoreKey(String::new()), block_ref);
            assert_eq!(request_type_of(&req), "view_state");
        }
    }

    #[test]
    fn view_access_key_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_access_key(
                "near".to_string(),
                "ed25519:abc".to_string(),
                block_ref,
            );
            assert_eq!(request_type_of(&req), "view_access_key");
        }
    }

    #[test]
    fn view_access_key_list_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_access_key_list("near".to_string(), block_ref);
            assert_eq!(request_type_of(&req), "view_access_key_list");
        }
    }

    #[test]
    fn view_gas_key_nonces_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_gas_key_nonces(
                "near".to_string(),
                "ed25519:abc".to_string(),
                block_ref,
            );
            assert_eq!(request_type_of(&req), "view_gas_key_nonces");
        }
    }

    #[test]
    fn call_function_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::call_function(
                "near".to_string(),
                "get_num",
                "e30=".to_string(),
                block_ref,
            );
            assert_eq!(request_type_of(&req), "call_function");
        }
    }

    #[test]
    fn view_global_contract_code_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_global_contract_code("abc123".to_string(), block_ref);
            assert_eq!(request_type_of(&req), "view_global_contract_code");
        }
    }

    #[test]
    fn view_global_contract_code_by_account_id_sets_request_type() {
        for block_ref in all_block_refs() {
            let req = RpcQueryRequest::view_global_contract_code_by_account_id(
                "near".to_string(),
                block_ref,
            );
            assert_eq!(
                request_type_of(&req),
                "view_global_contract_code_by_account_id"
            );
        }
    }

    // ── Round-trip JSON structure test ───────────────────────────────────

    #[test]
    fn view_account_finality_json_structure() {
        let req = RpcQueryRequest::view_account("near".to_string(), Finality::Final);
        let json: Value = serde_json::to_value(&req).expect("serialize");

        assert_eq!(json["request_type"], "view_account");
        assert_eq!(json["account_id"], "near");
        assert_eq!(json["finality"], "final");
        // block_id and sync_checkpoint should not be present
        assert!(json.get("block_id").is_none());
        assert!(json.get("sync_checkpoint").is_none());
    }

    #[test]
    fn view_account_block_id_json_structure() {
        let req = RpcQueryRequest::view_account("near".to_string(), BlockId::BlockHeight(12345));
        let json: Value = serde_json::to_value(&req).expect("serialize");

        assert_eq!(json["request_type"], "view_account");
        assert_eq!(json["account_id"], "near");
        assert_eq!(json["block_id"], 12345);
        assert!(json.get("finality").is_none());
        assert!(json.get("sync_checkpoint").is_none());
    }

    #[test]
    fn view_account_sync_checkpoint_json_structure() {
        let req =
            RpcQueryRequest::view_account("near".to_string(), SyncCheckpoint::EarliestAvailable);
        let json: Value = serde_json::to_value(&req).expect("serialize");

        assert_eq!(json["request_type"], "view_account");
        assert_eq!(json["account_id"], "near");
        assert_eq!(json["sync_checkpoint"], "earliest_available");
        assert!(json.get("finality").is_none());
        assert!(json.get("block_id").is_none());
    }

    #[test]
    fn call_function_json_structure() {
        let req = RpcQueryRequest::call_function(
            "contract.near".to_string(),
            "get_status",
            "e30=".to_string(),
            Finality::Optimistic,
        );
        let json: Value = serde_json::to_value(&req).expect("serialize");

        assert_eq!(json["request_type"], "call_function");
        assert_eq!(json["account_id"], "contract.near");
        assert_eq!(json["method_name"], "get_status");
        assert_eq!(json["args_base64"], "e30=");
        assert_eq!(json["finality"], "optimistic");
    }

    // ── Helpers ─────────────────────────────────────────────────────────

    fn all_block_refs() -> Vec<BlockReference> {
        vec![
            BlockReference::Finality(Finality::Final),
            BlockReference::BlockId(BlockId::BlockHeight(100)),
            BlockReference::SyncCheckpoint(SyncCheckpoint::Genesis),
        ]
    }
}
