use near_openrpc_client::types::*;

#[test]
fn rpc_query_request_serializes_with_request_type_field() {
    let req = RpcQueryRequest::ViewAccount(RpcViewAccountRequest::FinalityAccountId {
        account_id: "alice.near".parse().unwrap(),
        finality: Finality::Final,
    });
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["request_type"], "view_account");
    assert_eq!(json["account_id"], "alice.near");
    assert_eq!(json["finality"], "final");
}

#[test]
fn rpc_query_request_call_function_serializes_correctly() {
    let req = RpcQueryRequest::CallFunction(RpcCallFunctionRequest::FinalityAccountId {
        account_id: "contract.near".parse().unwrap(),
        method_name: "get_status".to_string(),
        args_base64: "e30=".parse().unwrap(),
        finality: Finality::Final,
    });
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["request_type"], "call_function");
    assert_eq!(json["method_name"], "get_status");
    assert_eq!(json["args_base64"], "e30=");
}

#[test]
fn rpc_query_request_view_access_key_serializes_correctly() {
    let req = RpcQueryRequest::ViewAccessKey(RpcViewAccessKeyRequest::FinalityAccountId {
        account_id: "alice.near".parse().unwrap(),
        public_key: "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
            .parse()
            .unwrap(),
        finality: Finality::Final,
    });
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["request_type"], "view_access_key");
    assert!(json["public_key"].as_str().unwrap().starts_with("ed25519:"));
}

#[test]
fn rpc_query_response_deserializes_view_account() {
    let json = serde_json::json!({
        "amount": "1000000000000000000000000",
        "block_hash": "9FMnGHBEfJ3PoKzSaq7EwCotanD3RLGA9UFqEjB3hrN1",
        "block_height": 12345,
        "code_hash": "11111111111111111111111111111111",
        "locked": "0",
        "storage_usage": 182,
        "storage_paid_at": 0
    });

    let resp: RpcQueryResponse = serde_json::from_value(json).unwrap();
    assert!(matches!(resp, RpcQueryResponse::ViewAccount(_)));
}

#[test]
fn rpc_query_response_deserializes_call_function() {
    let json = serde_json::json!({
        "block_hash": "9FMnGHBEfJ3PoKzSaq7EwCotanD3RLGA9UFqEjB3hrN1",
        "block_height": 12345,
        "logs": [],
        "result": [123, 125]
    });

    let resp: RpcQueryResponse = serde_json::from_value(json).unwrap();
    assert!(matches!(resp, RpcQueryResponse::CallFunction(_)));
}

#[test]
fn rpc_query_response_deserializes_view_code() {
    let json = serde_json::json!({
        "block_hash": "9FMnGHBEfJ3PoKzSaq7EwCotanD3RLGA9UFqEjB3hrN1",
        "block_height": 12345,
        "code_base64": "AGFzbQEAAAA=",
        "hash": "11111111111111111111111111111111"
    });

    let resp: RpcQueryResponse = serde_json::from_value(json).unwrap();
    assert!(matches!(resp, RpcQueryResponse::ViewCode(_)));
}

#[test]
fn rpc_query_response_deserializes_view_access_key_list() {
    let json = serde_json::json!({
        "block_hash": "9FMnGHBEfJ3PoKzSaq7EwCotanD3RLGA9UFqEjB3hrN1",
        "block_height": 12345,
        "keys": []
    });

    let resp: RpcQueryResponse = serde_json::from_value(json).unwrap();
    assert!(matches!(resp, RpcQueryResponse::ViewAccessKeyList(_)));
}
