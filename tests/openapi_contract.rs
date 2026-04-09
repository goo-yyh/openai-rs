use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
struct ContractEndpoint {
    method: String,
    template: String,
}

#[derive(Debug, Deserialize)]
struct ShapeAssertion {
    method: String,
    template: String,
    success_status: String,
    request_properties: Vec<String>,
    response_properties: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenApiContracts {
    documented_generated_exceptions: BTreeMap<String, String>,
    manual_endpoints: BTreeMap<String, ContractEndpoint>,
    shape_assertions: BTreeMap<String, ShapeAssertion>,
}

#[derive(Debug, Deserialize)]
struct SourceMetadata {
    documented_spec_version: String,
    openai_openapi_repo_head: String,
}

type EndpointManifest = BTreeMap<String, BTreeMap<String, ContractEndpoint>>;

fn repo_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn load_json<T>(path: &str) -> T
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(&fs::read_to_string(repo_file(path)).unwrap()).unwrap()
}

fn normalize_template(template: &str) -> String {
    let mut normalized = String::with_capacity(template.len());
    let mut chars = template.chars();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            normalized.push_str("{}");
            for next in chars.by_ref() {
                if next == '}' {
                    break;
                }
            }
        } else {
            normalized.push(ch);
        }
    }
    normalized
}

fn load_openapi_snapshot() -> Value {
    load_json("codegen/openapi/openapi.documented.json")
}

fn mapping(value: &Value) -> &Map<String, Value> {
    value.as_object().unwrap()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    mapping(value)
        .get(key)
        .unwrap_or_else(|| panic!("missing field `{key}`"))
}

fn maybe_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    mapping(value).get(key)
}

fn resolve_pointer<'a>(doc: &'a Value, reference: &str) -> &'a Value {
    let pointer = reference
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("unsupported ref: {reference}"));
    pointer
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(doc, |value, segment| {
            let segment = segment.replace("~1", "/").replace("~0", "~");
            match value {
                Value::Object(mapping) => mapping
                    .get(&segment)
                    .unwrap_or_else(|| panic!("unresolved ref segment `{segment}` in {reference}")),
                Value::Array(sequence) => {
                    let index = segment.parse::<usize>().unwrap_or_else(|_| {
                        panic!("invalid sequence ref segment `{segment}` in {reference}")
                    });
                    sequence.get(index).unwrap_or_else(|| {
                        panic!("unresolved ref segment `{segment}` in {reference}")
                    })
                }
                _ => panic!("unsupported ref segment `{segment}` in {reference}"),
            }
        })
}

fn resolve_schema<'a>(doc: &'a Value, mut schema: &'a Value) -> &'a Value {
    while let Some(reference) = maybe_field(schema, "$ref").and_then(Value::as_str) {
        schema = resolve_pointer(doc, reference);
    }
    schema
}

fn operation<'a>(doc: &'a Value, template: &str, method: &str) -> &'a Value {
    let normalized = normalize_template(template);
    let paths = mapping(field(doc, "paths"));
    let path_item = paths
        .iter()
        .find_map(|(path, value)| (normalize_template(path) == normalized).then_some(value))
        .unwrap_or_else(|| panic!("missing path in OpenAPI snapshot: {template}"));
    maybe_field(path_item, &method.to_ascii_lowercase())
        .unwrap_or_else(|| panic!("missing method `{method}` for path `{template}`"))
}

fn has_success_response(operation: &Value) -> bool {
    let responses = mapping(field(operation, "responses"));
    responses.keys().any(|status| {
        status
            .parse::<u16>()
            .map(|status| (200..300).contains(&status))
            .unwrap_or(false)
    })
}

fn response_schema<'a>(doc: &'a Value, operation: &'a Value, status: &str) -> &'a Value {
    let response = field(field(operation, "responses"), status);
    let content = mapping(field(response, "content"));
    let schema = content
        .get("application/json")
        .or_else(|| content.values().next())
        .and_then(|content| maybe_field(content, "schema"))
        .unwrap_or_else(|| panic!("missing response schema for status {status}"));
    resolve_schema(doc, schema)
}

fn request_schema<'a>(doc: &'a Value, operation: &'a Value) -> &'a Value {
    let request_body = field(operation, "requestBody");
    let content = mapping(field(request_body, "content"));
    let schema = content
        .get("application/json")
        .or_else(|| content.get("application/x-www-form-urlencoded"))
        .or_else(|| content.values().next())
        .and_then(|content| maybe_field(content, "schema"))
        .unwrap_or_else(|| panic!("missing request schema"));
    resolve_schema(doc, schema)
}

fn collect_properties(doc: &Value, schema: &Value, output: &mut BTreeSet<String>) {
    let schema = resolve_schema(doc, schema);
    if let Some(properties) = maybe_field(schema, "properties").and_then(Value::as_object) {
        for key in properties.keys() {
            output.insert(key.to_owned());
        }
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(items) = maybe_field(schema, combinator).and_then(Value::as_array) {
            for item in items {
                collect_properties(doc, item, output);
            }
        }
    }
}

#[test]
fn test_openapi_snapshot_metadata_should_match_expected_source() {
    let metadata: SourceMetadata = load_json("codegen/openapi/metadata.json");
    assert_eq!(metadata.documented_spec_version, "2.3.0");
    assert_eq!(
        metadata.openai_openapi_repo_head,
        "e1cb7a86ad53bb818c106ac7875e2a78182bb120"
    );
}

#[test]
fn test_documented_openapi_should_cover_generated_and_manual_endpoints() {
    let doc = load_openapi_snapshot();
    let manifest: EndpointManifest = load_json("codegen/endpoints.json");
    let contracts: OpenApiContracts = load_json("codegen/openapi/contracts.json");

    let generated = manifest
        .into_iter()
        .flat_map(|(_, endpoints)| endpoints.into_iter())
        .collect::<BTreeMap<_, _>>();

    for (endpoint_id, endpoint) in &generated {
        if contracts
            .documented_generated_exceptions
            .contains_key(endpoint_id)
        {
            continue;
        }

        let operation = operation(&doc, &endpoint.template, &endpoint.method);
        assert!(
            has_success_response(operation),
            "missing success response for generated endpoint `{endpoint_id}`"
        );
    }

    for endpoint_id in contracts.documented_generated_exceptions.keys() {
        let endpoint = generated.get(endpoint_id).unwrap_or_else(|| {
            panic!("exception references unknown generated endpoint `{endpoint_id}`")
        });
        let paths = mapping(field(&doc, "paths"));
        let normalized = normalize_template(&endpoint.template);
        let documented = paths.iter().any(|(path, value)| {
            normalize_template(path) == normalized
                && maybe_field(value, &endpoint.method.to_ascii_lowercase()).is_some()
        });
        assert!(
            !documented,
            "generated exception `{endpoint_id}` is now documented upstream and should be removed"
        );
    }

    for (endpoint_id, endpoint) in &contracts.manual_endpoints {
        let operation = operation(&doc, &endpoint.template, &endpoint.method);
        assert!(
            has_success_response(operation),
            "missing success response for manual endpoint `{endpoint_id}`"
        );
    }
}

#[test]
fn test_openapi_shape_assertions_should_match_high_value_sdk_contracts() {
    let doc = load_openapi_snapshot();
    let contracts: OpenApiContracts = load_json("codegen/openapi/contracts.json");

    for (endpoint_id, assertion) in &contracts.shape_assertions {
        let operation = operation(&doc, &assertion.template, &assertion.method);
        assert!(
            maybe_field(field(operation, "responses"), &assertion.success_status).is_some(),
            "missing success status {} for `{endpoint_id}`",
            assertion.success_status
        );

        let mut request_properties = BTreeSet::new();
        collect_properties(
            &doc,
            request_schema(&doc, operation),
            &mut request_properties,
        );
        for property in &assertion.request_properties {
            assert!(
                request_properties.contains(property),
                "missing request property `{property}` for `{endpoint_id}`"
            );
        }

        let mut response_properties = BTreeSet::new();
        collect_properties(
            &doc,
            response_schema(&doc, operation, &assertion.success_status),
            &mut response_properties,
        );
        for property in &assertion.response_properties {
            assert!(
                response_properties.contains(property),
                "missing response property `{property}` for `{endpoint_id}`"
            );
        }
    }
}
