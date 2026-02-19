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

    // Format with prettyplease
    let formatted = prettyplease_format(&code).unwrap_or(code);

    // Strip verbose JSON schema doc blocks and fix doctests
    let stripped = strip_json_schema_docs(&formatted);

    fs::write(out_path, stripped).expect("Failed to write generated.rs");
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
