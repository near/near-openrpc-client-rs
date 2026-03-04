use std::fs;
use std::path::Path;

fn main() {
    let openrpc_path = Path::new("openrpc.json");
    let out_path = Path::new("src/generated.rs");

    println!("cargo:rerun-if-changed={}", openrpc_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    // Read the OpenRPC spec
    let openrpc_content = fs::read_to_string(openrpc_path).expect("Failed to read openrpc.json");
    let openrpc: serde_json::Value =
        serde_json::from_str(&openrpc_content).expect("Failed to parse openrpc.json");

    // Extract the JSON Schema from OpenRPC's components.schemas
    let schemas = openrpc
        .get("components")
        .and_then(|c| c.get("schemas"))
        .expect("OpenRPC must have components.schemas");

    // Create a JSON Schema document with definitions
    let json_schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "definitions": schemas
    });

    // Convert $refs from OpenRPC format to JSON Schema format
    let json_schema_str = serde_json::to_string(&json_schema)
        .expect("Failed to serialize schema")
        .replace("#/components/schemas/", "#/definitions/");

    let schema: serde_json::Value =
        serde_json::from_str(&json_schema_str).expect("Failed to parse converted schema");

    // Generate Rust types with typify
    let mut type_space = typify::TypeSpace::default();
    type_space
        .add_root_schema(serde_json::from_value(schema).expect("Failed to convert schema"))
        .expect("Failed to add schema to type space");

    let code = type_space.to_stream().to_string();

    // Generate unified RpcQueryRequest/RpcQueryResponse enums from query-tagged methods
    let query_enums = generate_query_enums(&openrpc);

    let combined = format!("{code}\n{query_enums}");

    // Format with prettyplease
    let formatted = prettyplease_format(&combined).unwrap_or(combined);

    // Strip verbose JSON schema doc blocks and fix doctests
    let stripped = strip_json_schema_docs(&formatted);

    fs::write(out_path, stripped).expect("Failed to write generated.rs");
}

/// Convert a snake_case string to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    let mut s = c.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Parse query-tagged methods from the OpenRPC spec and generate unified
/// `RpcQueryRequest` and `RpcQueryResponse` enums.
///
/// NEAR's `query` RPC method uses a `request_type` discriminant field. The OpenRPC
/// spec models each query variant as a separate `EXPERIMENTAL_*` method tagged with
/// "query". This function collects those methods and emits enums that serialize with
/// `request_type` (for the request) and deserialize as untagged (for the response).
fn generate_query_enums(openrpc: &serde_json::Value) -> String {
    let methods = openrpc
        .get("methods")
        .and_then(|m| m.as_array())
        .expect("OpenRPC must have methods array");

    // Collect (PascalName, RequestTypeName, ResponseTypeName) for query-tagged methods
    let mut variants: Vec<(String, String, String)> = Vec::new();

    for method in methods {
        let tags = method.get("tags").and_then(|t| t.as_array());
        let is_query = tags
            .map(|tags| {
                tags.iter()
                    .any(|t| t.get("name").and_then(|n| n.as_str()) == Some("query"))
            })
            .unwrap_or(false);

        if !is_query {
            continue;
        }

        let name = method
            .get("name")
            .and_then(|n| n.as_str())
            .expect("method must have name");

        // Strip EXPERIMENTAL_ prefix and convert to PascalCase
        let snake_name = name.strip_prefix("EXPERIMENTAL_").unwrap_or(name);
        let pascal_name = to_pascal_case(snake_name);

        let request_type = format!("Rpc{pascal_name}Request");
        let response_type = format!("Rpc{pascal_name}Response");

        variants.push((pascal_name, request_type, response_type));
    }

    if variants.is_empty() {
        return String::new();
    }

    // Build RpcQueryRequest enum (internally tagged by request_type)
    let mut code = String::new();
    code.push_str(
        "/// Unified request enum for NEAR's polymorphic `query` JSON-RPC method.\n\
         ///\n\
         /// Serializes with a `request_type` discriminant field in snake_case,\n\
         /// matching what the NEAR RPC node expects.\n\
         #[derive(::serde::Serialize, Clone, Debug)]\n\
         #[serde(tag = \"request_type\", rename_all = \"snake_case\")]\n\
         pub enum RpcQueryRequest {\n",
    );
    for (pascal, req_type, _) in &variants {
        code.push_str(&format!("    {pascal}({req_type}),\n"));
    }
    code.push_str("}\n\n");

    // Build RpcQueryResponse enum (untagged, try each variant in order)
    code.push_str(
        "/// Unified response enum for NEAR's polymorphic `query` JSON-RPC method.\n\
         ///\n\
         /// Deserializes as untagged since the NEAR RPC node does not include a\n\
         /// discriminant in query responses.\n\
         #[derive(::serde::Deserialize, Clone, Debug)]\n\
         #[serde(untagged)]\n\
         pub enum RpcQueryResponse {\n",
    );
    // CallFunction first — it has a unique `result` field, so placing it before
    // structs that share common fields avoids ambiguous untagged matches.
    for (pascal, _, resp_type) in &variants {
        code.push_str(&format!("    {pascal}({resp_type}),\n"));
    }
    code.push_str("}\n");

    code
}

fn prettyplease_format(code: &str) -> Option<String> {
    let syntax_tree = syn::parse_file(code).ok()?;
    Some(prettyplease::unparse(&syntax_tree))
}

/// Strip JSON schema documentation blocks from generated code.
///
/// Removes collapsible `<details>` blocks containing raw JSON schemas that bloat
/// the generated file. Also marks code examples with `ignore` to prevent doctest
/// failures on external crate references.
fn strip_json_schema_docs(code: &str) -> String {
    let mut result = Vec::new();
    let mut in_details_block = false;

    for line in code.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("///")
            && trimmed.contains("<details>")
            && trimmed.contains("JSON schema")
        {
            in_details_block = true;
            continue;
        }

        if in_details_block && trimmed.starts_with("///") && trimmed.contains("</details>") {
            in_details_block = false;
            continue;
        }

        if in_details_block {
            continue;
        }

        if trimmed == "/// ```" || trimmed == "///```" || trimmed == "```" {
            result.push(line.replace("```", "```ignore"));
        } else {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}
