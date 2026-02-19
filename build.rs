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

    let mut schema: serde_json::Value =
        serde_json::from_str(&json_schema_str).expect("Failed to parse converted schema");

    // Preprocess: expand allOf[$ref, $ref] into oneOf with cartesian product.
    // This helps typify generate meaningful enum variant names (e.g. ViewAccountFinality)
    // instead of opaque names like Variant0, Variant1.
    expand_allof_refs(&mut schema);

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

/// Expand allOf schemas containing only $ref items into oneOf with cartesian product.
///
/// When the OpenRPC spec has `allOf: [$ref_A, $ref_B]` where both refs point to
/// `oneOf` enums, compute the cartesian product of their variants and merge properties.
/// This produces better-named enum variants in typify output.
fn expand_allof_refs(schema: &mut serde_json::Value) {
    if let serde_json::Value::Object(obj) = schema {
        for (_, v) in obj.iter_mut() {
            expand_allof_refs(v);
        }

        if let Some(serde_json::Value::Object(defs)) = obj.get_mut("definitions") {
            let def_keys: Vec<String> = defs.keys().cloned().collect();

            for key in def_keys {
                if let Some(def) = defs.get(&key).cloned() {
                    if let Some(expanded) = try_expand_allof(&def, defs) {
                        defs.insert(key, expanded);
                    }
                }
            }
        }
    }
}

/// Try to expand an allOf schema into oneOf with cartesian product.
/// Returns None if not applicable.
fn try_expand_allof(
    schema: &serde_json::Value,
    definitions: &serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    let obj = schema.as_object()?;
    let all_of = obj.get("allOf")?.as_array()?;

    // Check if all items are $ref
    let refs: Vec<&str> = all_of
        .iter()
        .filter_map(|item| {
            item.as_object()
                .and_then(|o| o.get("$ref"))
                .and_then(|r| r.as_str())
        })
        .collect();

    if refs.len() != all_of.len() || refs.len() < 2 {
        return None;
    }

    // Resolve each ref and get its oneOf variants (or treat as single variant)
    let mut variant_groups: Vec<Vec<serde_json::Value>> = Vec::new();

    for ref_path in &refs {
        let ref_name = ref_path.strip_prefix("#/definitions/")?;
        let ref_schema = definitions.get(ref_name)?;

        let variants = if let Some(one_of) = ref_schema.get("oneOf").and_then(|v| v.as_array()) {
            one_of.clone()
        } else {
            vec![ref_schema.clone()]
        };

        variant_groups.push(variants);
    }

    // Generate cartesian product
    let mut combined_variants = vec![serde_json::json!({})];

    for group in variant_groups {
        let mut new_combined = Vec::new();
        for existing in &combined_variants {
            for variant in &group {
                if let Some(merged) = merge_variant_properties(existing, variant) {
                    new_combined.push(merged);
                }
            }
        }
        combined_variants = new_combined;
    }

    let title = obj.get("title").cloned();
    let mut result = serde_json::json!({
        "oneOf": combined_variants
    });

    if let Some(t) = title {
        result
            .as_object_mut()
            .unwrap()
            .insert("title".to_string(), t);
    }

    Some(result)
}

/// Merge properties from two variant objects into one.
fn merge_variant_properties(
    a: &serde_json::Value,
    b: &serde_json::Value,
) -> Option<serde_json::Value> {
    let a_obj = a.as_object()?;
    let b_obj = b.as_object()?;

    let mut result = serde_json::Map::new();

    // Merge properties
    let mut props = serde_json::Map::new();
    if let Some(a_props) = a_obj.get("properties").and_then(|p| p.as_object()) {
        for (k, v) in a_props {
            props.insert(k.clone(), v.clone());
        }
    }
    if let Some(b_props) = b_obj.get("properties").and_then(|p| p.as_object()) {
        for (k, v) in b_props {
            props.insert(k.clone(), v.clone());
        }
    }
    if !props.is_empty() {
        result.insert("properties".to_string(), serde_json::Value::Object(props));
    }

    // Merge required arrays
    let mut required: Vec<String> = Vec::new();
    if let Some(a_req) = a_obj.get("required").and_then(|r| r.as_array()) {
        for r in a_req {
            if let Some(s) = r.as_str() {
                if !required.contains(&s.to_string()) {
                    required.push(s.to_string());
                }
            }
        }
    }
    if let Some(b_req) = b_obj.get("required").and_then(|r| r.as_array()) {
        for r in b_req {
            if let Some(s) = r.as_str() {
                if !required.contains(&s.to_string()) {
                    required.push(s.to_string());
                }
            }
        }
    }
    if !required.is_empty() {
        result.insert("required".to_string(), serde_json::json!(required));
    }

    // Generate a title from combined variant titles
    let a_title = a_obj.get("title").and_then(|t| t.as_str()).unwrap_or("");
    let b_title = b_obj.get("title").and_then(|t| t.as_str()).unwrap_or("");
    if !a_title.is_empty() || !b_title.is_empty() {
        let combined_title = if a_title.is_empty() {
            b_title.to_string()
        } else if b_title.is_empty() {
            a_title.to_string()
        } else {
            format!("{}{}", b_title, a_title)
        };
        result.insert("title".to_string(), serde_json::json!(combined_title));
    }

    result.insert("type".to_string(), serde_json::json!("object"));

    Some(serde_json::Value::Object(result))
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
