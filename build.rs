use std::collections::{BTreeMap, HashMap};
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

    // Preprocess: convert `"const"` string properties into single-variant enums with a default.
    // This makes typify generate a type with `Default` so users don't have to set fields like
    // `request_type` manually — the const value is filled in automatically.
    convert_const_to_defaulted_enum(&mut schema);

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

    // Post-process: remove `request_type` fields from enum variants and generate custom
    // Serialize impls that inject the correct const value automatically.
    let final_code = elide_const_request_type_fields(&stripped);

    fs::write(out_path, final_code).expect("Failed to write generated.rs");
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
                if let Some(def) = defs.get(&key).cloned()
                    && let Some(expanded) = try_expand_allof(&def, defs)
                {
                    defs.insert(key, expanded);
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
            if let Some(s) = r.as_str()
                && !required.contains(&s.to_string())
            {
                required.push(s.to_string());
            }
        }
    }
    if let Some(b_req) = b_obj.get("required").and_then(|r| r.as_array()) {
        for r in b_req {
            if let Some(s) = r.as_str()
                && !required.contains(&s.to_string())
            {
                required.push(s.to_string());
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

/// Convert the `request_type` `"const"` property into a single-variant enum with a default.
///
/// Only targets the `request_type` field specifically. When a JSON Schema property has
/// `{"const": "some_value", "type": "string"}`, typify ignores the `const` and generates
/// a plain `String` field. By converting it to
/// `{"enum": ["some_value"], "type": "string", "default": "some_value"}`, typify generates
/// a single-variant enum type with a `Default` impl. Combined with removing the property
/// from `required`, this lets users omit the field during construction — serde fills it
/// in automatically via `#[serde(default)]`.
///
/// This is intentionally restricted to `request_type` to avoid making other const
/// discriminator fields (e.g. `type` in `StateChangeCauseView`) optional, which would
/// weaken deserialization strictness.
fn convert_const_to_defaulted_enum(schema: &mut serde_json::Value) {
    match schema {
        serde_json::Value::Object(obj) => {
            // Check if this object has a "request_type" property with a const value
            if let Some(serde_json::Value::Object(props)) = obj.get_mut("properties")
                && let Some(prop) = props.get_mut("request_type")
                && let Some(prop_obj) = prop.as_object_mut()
                && let Some(const_val) = prop_obj.get("const").cloned()
            {
                // Replace {"const": "val", "type": "string"}
                // with {"enum": ["val"], "type": "string", "default": "val"}
                prop_obj.remove("const");
                prop_obj.insert(
                    "enum".to_string(),
                    serde_json::Value::Array(vec![const_val.clone()]),
                );
                prop_obj.insert("default".to_string(), const_val);

                // Remove request_type from `required` so serde uses the default
                if let Some(serde_json::Value::Array(required)) = obj.get_mut("required") {
                    required.retain(|r| r.as_str() != Some("request_type"));
                }
            }

            // Recurse into all values
            for (_, v) in obj.iter_mut() {
                convert_const_to_defaulted_enum(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                convert_const_to_defaulted_enum(v);
            }
        }
        _ => {}
    }
}

/// Remove `request_type` fields from enum variants and generate custom `Serialize` impls
/// that inject the correct const value during serialization.
///
/// After `convert_const_to_defaulted_enum` transforms the schema, typify generates enum
/// variants with `request_type: SomeRequestType` fields and `#[serde(default = "...")]`.
/// This function goes further: it removes those fields entirely so users don't need to
/// specify them during construction, and generates `Serialize` impls that inject the
/// correct const value into the JSON output.
fn elide_const_request_type_fields(code: &str) -> String {
    // Step 1: Build a map from RequestType type names to their serde rename (const) values.
    let request_type_values = extract_request_type_values(code);

    // Step 2: For each target enum, collect variant info, remove request_type fields,
    // and generate a custom Serialize impl.
    let mut result = code.to_string();

    for enum_name in &["RpcQueryRequest", "QueryRequest"] {
        if let Some(processed) =
            process_enum_request_type(&result, enum_name, &request_type_values)
        {
            result = processed;
        }
    }

    result
}

/// Extract a mapping from RequestType type names to their const string values.
///
/// Scans for single-variant enums like:
/// ```ignore
/// pub enum ViewAccountBlockIdRequestType {
///     #[serde(rename = "view_account")]
///     ViewAccount,
/// }
/// ```
fn extract_request_type_values(code: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let lines: Vec<&str> = code.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("pub enum ")
            && let Some(name) = rest.strip_suffix(" {")
            && name.ends_with("RequestType")
        {
            for inner_line in lines.iter().skip(i + 1).take(4) {
                let inner = inner_line.trim();
                if let Some(attr_rest) = inner.strip_prefix("#[serde(rename = \"")
                    && let Some(value) = attr_rest.strip_suffix("\")]")
                {
                    map.insert(name.to_string(), value.to_string());
                    break;
                }
            }
        }
    }

    map
}

/// Parsed info about an enum variant's fields.
struct VariantInfo {
    name: String,
    /// The const value for request_type (if it has one).
    request_type_const: String,
    /// Fields other than request_type, as (name, type, serde_attributes) tuples.
    fields: Vec<VariantField>,
}

struct VariantField {
    name: String,
    type_str: String,
    /// Any `#[serde(...)]` attributes on the field.
    serde_attrs: Vec<String>,
}

/// Process a single enum: remove `request_type` fields and generate a custom Serialize impl.
fn process_enum_request_type(
    code: &str,
    enum_name: &str,
    request_type_values: &HashMap<String, String>,
) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    let enum_pattern = format!("pub enum {enum_name} {{");
    let enum_start = lines.iter().position(|l| l.trim() == enum_pattern)?;

    // Find the derive line
    let mut derive_line = None;
    for i in (0..enum_start).rev() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("#[derive(") {
            derive_line = Some(i);
            break;
        }
        if !trimmed.is_empty()
            && !trimmed.starts_with("///")
            && !trimmed.starts_with("#[")
            && !trimmed.starts_with("//")
        {
            break;
        }
    }
    let derive_line = derive_line?;

    // Find the enum's closing brace
    let mut brace_depth = 0;
    let mut enum_end = enum_start;
    for (i, line) in lines.iter().enumerate().skip(enum_start) {
        for ch in line.chars() {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    enum_end = i;
                    break;
                }
            }
        }
        if brace_depth == 0 && i > enum_start {
            break;
        }
    }

    // Parse variant info
    let variants = parse_enum_variants(&lines, enum_start + 1, enum_end, request_type_values);

    if variants.is_empty() {
        return None;
    }

    // Rebuild the code
    let mut new_lines: Vec<String> = Vec::new();

    // Lines before the derive
    new_lines.extend(lines[..derive_line].iter().map(|l| l.to_string()));

    // Remove both Serialize and Deserialize from derive (we impl both manually)
    let modified_derive = lines[derive_line]
        .replace("::serde::Deserialize, ::serde::Serialize, ", "")
        .replace("::serde::Deserialize, ", "")
        .replace(", ::serde::Deserialize", "")
        .replace("::serde::Serialize, ", "")
        .replace(", ::serde::Serialize", "")
        .replace("::serde::Serialize", "")
        .replace("::serde::Deserialize", "");
    new_lines.push(modified_derive);

    // Lines between derive and enum body opening, but remove #[serde(untagged)]
    for line in &lines[derive_line + 1..=enum_start] {
        let trimmed = line.trim();
        if trimmed == "#[serde(untagged)]" {
            continue;
        }
        new_lines.push(line.to_string());
    }

    // Rebuild enum body without request_type fields and without serde attributes
    // (since we removed derive(Serialize, Deserialize), serde attributes would be invalid)
    for variant in &variants {
        new_lines.push(format!("    {} {{", variant.name));
        for field in &variant.fields {
            // Skip serde attributes — our custom impls handle serialization logic
            new_lines.push(format!("        {}: {},", field.name, field.type_str));
        }
        new_lines.push("    },".to_string());
    }

    // Close enum
    new_lines.push("}".to_string());

    // Generate custom Serialize and Deserialize impls
    new_lines.push(String::new());
    new_lines.push(generate_serialize_impl(enum_name, &variants));
    new_lines.push(String::new());
    new_lines.push(generate_deserialize_impl(enum_name, &variants));

    // Copy remaining lines after the original enum
    new_lines.extend(lines[enum_end + 1..].iter().map(|l| l.to_string()));

    let joined = new_lines.join("\n");
    prettyplease_format(&joined).or(Some(joined))
}

/// Parse enum variants from the generated code, extracting field info.
fn parse_enum_variants(
    lines: &[&str],
    start: usize,
    end: usize,
    request_type_values: &HashMap<String, String>,
) -> Vec<VariantInfo> {
    let mut variants = Vec::new();
    let mut i = start;

    while i < end {
        let trimmed = lines[i].trim();

        // Skip doc comments and attributes before variant name
        if trimmed.starts_with("///") || trimmed.starts_with("#[") || trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Match variant name: "VariantName {"
        if let Some(variant_name) = trimmed.strip_suffix(" {") {
            let mut fields = Vec::new();
            let mut request_type_const = None;
            i += 1;

            // Parse fields until closing },
            let mut pending_serde_attrs: Vec<String> = Vec::new();
            while i < end {
                let field_trimmed = lines[i].trim();

                if field_trimmed == "}," || field_trimmed == "}" {
                    i += 1;
                    break;
                }

                // Collect serde attributes
                if field_trimmed.starts_with("#[serde(") {
                    let mut attr = field_trimmed.to_string();
                    if !field_trimmed.ends_with(")]") {
                        // Multi-line attribute
                        i += 1;
                        while i < end && !lines[i].trim().ends_with(")]") {
                            attr.push(' ');
                            attr.push_str(lines[i].trim());
                            i += 1;
                        }
                        if i < end {
                            attr.push(' ');
                            attr.push_str(lines[i].trim());
                        }
                    }
                    pending_serde_attrs.push(attr);
                    i += 1;
                    continue;
                }

                // Parse field: "field_name: Type,"
                if let Some(colon_pos) = field_trimmed.find(": ") {
                    let field_name = &field_trimmed[..colon_pos];
                    let type_with_comma = &field_trimmed[colon_pos + 2..];
                    let type_str = type_with_comma.trim_end_matches(',');

                    if field_name == "request_type" {
                        // Check if this is a known const request type
                        if let Some(const_val) = request_type_values.get(type_str) {
                            request_type_const = Some(const_val.clone());
                            pending_serde_attrs.clear();
                            i += 1;
                            continue;
                        }
                    }

                    fields.push(VariantField {
                        name: field_name.to_string(),
                        type_str: type_str.to_string(),
                        serde_attrs: std::mem::take(&mut pending_serde_attrs),
                    });
                }

                i += 1;
            }

            if let Some(const_val) = request_type_const {
                variants.push(VariantInfo {
                    name: variant_name.to_string(),
                    request_type_const: const_val,
                    fields,
                });
            }
        } else {
            i += 1;
        }
    }

    variants
}

/// Generate a custom `Serialize` impl that serializes each variant's fields as a flat map
/// and injects the `request_type` field with the correct const value.
fn generate_serialize_impl(enum_name: &str, variants: &[VariantInfo]) -> String {
    let mut match_arms = String::new();

    for variant in variants {
        let field_names: Vec<&str> = variant.fields.iter().map(|f| f.name.as_str()).collect();
        let bindings = field_names.join(", ");
        let field_count = field_names.len() + 1; // +1 for request_type

        let mut serialize_fields = String::new();
        for field in &variant.fields {
            // Check if the field has skip_serializing_if
            let has_skip = field
                .serde_attrs
                .iter()
                .any(|a| a.contains("skip_serializing_if"));

            if has_skip {
                // For Option fields with skip_serializing_if, only serialize if Some
                serialize_fields.push_str(&format!(
                    "            if {name}.is_some() {{\n                map.serialize_entry(\"{name}\", {name})?;\n            }}\n",
                    name = field.name,
                ));
            } else {
                serialize_fields.push_str(&format!(
                    "            map.serialize_entry(\"{name}\", {name})?;\n",
                    name = field.name,
                ));
            }
        }

        match_arms.push_str(&format!(
            r#"            {enum_name}::{variant_name} {{ {bindings} }} => {{
                let mut map = serializer.serialize_map(::std::option::Option::Some({field_count}))?;
{serialize_fields}            map.serialize_entry("request_type", "{const_value}")?;
                map.end()
            }}
"#,
            enum_name = enum_name,
            variant_name = variant.name,
            bindings = bindings,
            field_count = field_count,
            serialize_fields = serialize_fields,
            const_value = variant.request_type_const,
        ));
    }

    format!(
        r#"impl ::serde::Serialize for {enum_name} {{
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {{
        use ::serde::ser::SerializeMap;
        match self {{
{match_arms}        }}
    }}
}}"#,
        enum_name = enum_name,
        match_arms = match_arms,
    )
}

/// Generate a custom `Deserialize` impl that uses `request_type` as a discriminator.
///
/// Since we removed `request_type` from the enum variant fields, serde's untagged
/// deserialization can't distinguish variants with the same field structure. This impl
/// first extracts `request_type` from the JSON, then uses it plus the present fields
/// to pick the correct variant.
fn generate_deserialize_impl(enum_name: &str, variants: &[VariantInfo]) -> String {
    // Group variants by request_type value (BTreeMap for deterministic codegen output)
    let mut variants_by_rt: BTreeMap<&str, Vec<&VariantInfo>> = BTreeMap::new();
    for v in variants {
        variants_by_rt
            .entry(v.request_type_const.as_str())
            .or_default()
            .push(v);
    }

    // Generate match arms for each request_type value
    let mut rt_arms = String::new();
    for (rt_value, rt_variants) in &variants_by_rt {
        if rt_variants.len() == 1 {
            // Single variant for this request_type — straightforward
            let v = rt_variants[0];
            let field_extractions = generate_field_extractions(&v.fields, enum_name);
            rt_arms.push_str(&format!(
                "                \"{rt_value}\" => {{\n{field_extractions}                    Ok({enum_name}::{variant_name} {{ {field_names} }})\n                }}\n",
                rt_value = rt_value,
                enum_name = enum_name,
                variant_name = v.name,
                field_extractions = field_extractions,
                field_names = v.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "),
            ));
        } else {
            // Multiple variants share this request_type — discriminate by which extra
            // fields are present (e.g., block_id vs finality vs sync_checkpoint)
            let mut inner_arms = String::new();
            for (idx, v) in rt_variants.iter().enumerate() {
                let field_extractions = generate_field_extractions(&v.fields, enum_name);
                let discriminating_fields: Vec<&str> = v
                    .fields
                    .iter()
                    .filter(|f| {
                        // Fields that aren't present in ALL variants of this request_type
                        !rt_variants.iter().all(|other| {
                            other.fields.iter().any(|of| of.name == f.name)
                        })
                    })
                    .map(|f| f.name.as_str())
                    .collect();

                let condition = if !discriminating_fields.is_empty() {
                    discriminating_fields
                        .iter()
                        .map(|f| format!("map.contains_key(\"{f}\")"))
                        .collect::<Vec<_>>()
                        .join(" && ")
                } else if idx == rt_variants.len() - 1 {
                    // Last variant is the fallback
                    "true".to_string()
                } else {
                    "true".to_string()
                };

                if idx == 0 {
                    inner_arms.push_str(&format!(
                        "                    if {condition} {{\n{field_extractions}                        Ok({enum_name}::{variant_name} {{ {field_names} }})\n",
                        condition = condition,
                        enum_name = enum_name,
                        variant_name = v.name,
                        field_extractions = field_extractions,
                        field_names = v.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "),
                    ));
                } else if idx == rt_variants.len() - 1 {
                    inner_arms.push_str(&format!(
                        "                    }} else {{\n{field_extractions}                        Ok({enum_name}::{variant_name} {{ {field_names} }})\n                    }}\n",
                        enum_name = enum_name,
                        variant_name = v.name,
                        field_extractions = field_extractions,
                        field_names = v.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "),
                    ));
                } else {
                    inner_arms.push_str(&format!(
                        "                    }} else if {condition} {{\n{field_extractions}                        Ok({enum_name}::{variant_name} {{ {field_names} }})\n",
                        condition = condition,
                        enum_name = enum_name,
                        variant_name = v.name,
                        field_extractions = field_extractions,
                        field_names = v.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "),
                    ));
                }
            }

            rt_arms.push_str(&format!(
                "                \"{rt_value}\" => {{\n{inner_arms}                }}\n",
                rt_value = rt_value,
                inner_arms = inner_arms,
            ));
        }
    }

    format!(
        r#"impl<'de> ::serde::Deserialize<'de> for {enum_name} {{
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {{
        let map: serde_json::Map<::std::string::String, serde_json::Value> =
            serde_json::Map::deserialize(deserializer)?;

        let request_type = map
            .get("request_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ::serde::de::Error::missing_field("request_type"))?;

        match request_type {{
{rt_arms}                other => Err(::serde::de::Error::unknown_variant(
                    other,
                    &[{known_variants}],
                )),
        }}
    }}
}}"#,
        enum_name = enum_name,
        rt_arms = rt_arms,
        known_variants = variants_by_rt
            .keys()
            .map(|k| format!("\"{k}\""))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// Generate field extraction code from a serde_json::Map for a variant's fields.
fn generate_field_extractions(fields: &[VariantField], _enum_name: &str) -> String {
    let mut code = String::new();

    for field in fields {
        let is_optional = field
            .serde_attrs
            .iter()
            .any(|a| a.contains("skip_serializing_if"));

        if is_optional {
            code.push_str(&format!(
                "                    let {name} = map.get(\"{name}\").cloned().map(serde_json::from_value).transpose().map_err(::serde::de::Error::custom)?;\n",
                name = field.name,
            ));
        } else {
            code.push_str(&format!(
                "                    let {name} = map.get(\"{name}\").cloned().ok_or_else(|| ::serde::de::Error::missing_field(\"{name}\")).and_then(|v| serde_json::from_value(v).map_err(::serde::de::Error::custom))?;\n",
                name = field.name,
            ));
        }
    }

    code
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
