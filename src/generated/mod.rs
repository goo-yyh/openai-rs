pub(crate) mod endpoints;

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde::Deserialize;

    use super::endpoints::ALL_ENDPOINTS;

    #[derive(Debug, Deserialize)]
    struct EndpointSpec {
        method: String,
        template: String,
    }

    type EndpointManifest = BTreeMap<String, BTreeMap<String, EndpointSpec>>;

    fn extract_placeholders(template: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut start = None;
        for (index, byte) in template.bytes().enumerate() {
            match byte {
                b'{' => {
                    assert!(
                        start.is_none(),
                        "nested placeholder in template: {template}"
                    );
                    start = Some(index + 1);
                }
                b'}' => {
                    let placeholder_start = start.take().unwrap_or_else(|| {
                        panic!("unmatched closing brace in template: {template}")
                    });
                    placeholders.push(template[placeholder_start..index].to_owned());
                }
                _ => {}
            }
        }
        assert!(
            start.is_none(),
            "unmatched opening brace in template: {template}"
        );
        placeholders
    }

    #[test]
    fn test_generated_catalog_should_match_manifest() {
        let manifest: EndpointManifest =
            serde_json::from_str(include_str!("../../codegen/endpoints.json")).unwrap();
        let expected = manifest
            .into_iter()
            .flat_map(|(_, endpoints)| endpoints.into_iter())
            .map(|(id, spec)| (id, (spec.method, spec.template)))
            .collect::<BTreeMap<_, _>>();
        let actual = ALL_ENDPOINTS
            .iter()
            .map(|endpoint| {
                (
                    endpoint.id.to_owned(),
                    (endpoint.method.to_owned(), endpoint.template.to_owned()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_generated_catalog_templates_should_be_well_formed() {
        let mut seen_ids = BTreeSet::new();

        for endpoint in ALL_ENDPOINTS {
            assert!(
                endpoint.template.starts_with('/'),
                "template should start with '/': {}",
                endpoint.template
            );
            assert!(
                matches!(endpoint.method, "GET" | "POST" | "DELETE"),
                "unsupported method in generated catalog: {} {}",
                endpoint.method,
                endpoint.id
            );
            assert!(
                seen_ids.insert(endpoint.id),
                "duplicate endpoint id: {}",
                endpoint.id
            );

            let placeholders = extract_placeholders(endpoint.template);
            let unique_placeholders = placeholders.iter().collect::<BTreeSet<_>>();
            assert_eq!(
                unique_placeholders.len(),
                placeholders.len(),
                "duplicate placeholders in template: {}",
                endpoint.template
            );

            let owned_params = placeholders
                .iter()
                .map(|name| (name.clone(), format!("value_for_{name}")))
                .collect::<Vec<_>>();
            let borrowed_params = owned_params
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect::<Vec<_>>();
            let rendered = endpoint.render(&borrowed_params);
            assert!(
                !rendered.contains('{') && !rendered.contains('}'),
                "rendered path still contains unresolved placeholders: {rendered}"
            );
        }
    }
}
