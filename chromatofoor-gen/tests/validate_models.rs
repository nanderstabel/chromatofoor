#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const CANONICAL_TESTBEDS: &[&str] = &[
    "testbed/01-bank-account",
    "testbed/02-ecommerce-checkout",
    "testbed/03-course-enrollment",
    "testbed/04-flight-booking",
    "testbed/05-cargo-shipping",
];

const EXPECTED_KINDS: &[&str] = &[
    "actor",
    "aggregate",
    "aggregate-lifecycle",
    "bounded-context",
    "command",
    "context-mapping",
    "domain",
    "domain-service",
    "dynamic-consistency-boundary",
    "entity",
    "event",
    "event-handler",
    "external-system",
    "feature",
    "policy",
    "presentation-binding",
    "process-manager",
    "query",
    "read-model",
    "subdomain",
    "value-object",
];

const EXPECTED_TRANSPORTS: &[&str] = &["cli", "graphql", "http-rest", "mcp", "tauri-ipc"];

#[test]
fn canonical_testbeds_parse_strictly_have_descriptions_pass_lint_and_cover_all_esdm_kinds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");

    let mut missing_descriptions = Vec::new();
    let mut lint_failures = Vec::new();
    let mut seen_kinds = BTreeSet::new();
    let mut seen_transports = BTreeSet::new();

    for rel_dir in CANONICAL_TESTBEDS {
        let dir = root.join(rel_dir);
        assert!(
            dir.is_dir(),
            "Missing canonical testbed directory: {}",
            dir.display()
        );

        let mut yaml_files: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let name = path.file_name()?.to_str()?;
                if path.is_file() && name.ends_with(".esdm.yaml") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        yaml_files.sort();
        assert!(
            !yaml_files.is_empty(),
            "No *.esdm.yaml files found in {}",
            dir.display()
        );

        let mut testbed_docs = Vec::new();
        for path in yaml_files {
            let content = fs::read_to_string(&path).unwrap();
            for (idx, doc_de) in serde_yaml::Deserializer::from_str(&content).enumerate() {
                let val = serde_yaml::Value::deserialize(doc_de).unwrap_or_else(|e| {
                    panic!(
                        "YAML syntax error in {} (doc #{}): {e}",
                        path.display(),
                        idx + 1
                    )
                });
                if val.is_null() {
                    continue;
                }
                let doc: esdm::EsdmDoc = serde_yaml::from_value(val.clone()).unwrap_or_else(|e| {
                    panic!(
                        "EsdmDoc deserialization error in {} (doc #{}): {e}\nValue: {val:?}",
                        path.display(),
                        idx + 1
                    )
                });

                if doc
                    .description
                    .as_deref()
                    .is_none_or(|d| d.trim().is_empty())
                {
                    missing_descriptions.push(format!(
                        "{} -> doc #{}: kind={}, name={}",
                        path.display(),
                        idx + 1,
                        doc.kind,
                        doc.name
                    ));
                }

                seen_kinds.insert(doc.kind.clone());
                if doc.kind == "presentation-binding"
                    && let Some(ref transport) = doc.transport
                {
                    seen_transports.insert(transport.clone());
                }

                testbed_docs.push(doc);
            }
        }

        for violation in esdm::lint::validate_model(&testbed_docs) {
            lint_failures.push(format!("{rel_dir}: {violation}"));
        }
    }

    assert!(
        missing_descriptions.is_empty(),
        "Found {} ESDM documents missing a description:\n{}",
        missing_descriptions.len(),
        missing_descriptions.join("\n")
    );

    assert!(
        lint_failures.is_empty(),
        "Found {} ESDM model lint violations:\n{}",
        lint_failures.len(),
        lint_failures.join("\n")
    );

    let expected_kind_set: BTreeSet<String> =
        EXPECTED_KINDS.iter().map(|s| (*s).to_string()).collect();
    let missing_kinds: Vec<_> = expected_kind_set.difference(&seen_kinds).collect();
    assert!(
        missing_kinds.is_empty(),
        "Canonical testbeds 01-05 are missing ESDM kinds: {missing_kinds:?}"
    );

    let expected_transport_set: BTreeSet<String> = EXPECTED_TRANSPORTS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let missing_transports: Vec<_> = expected_transport_set
        .difference(&seen_transports)
        .collect();
    assert!(
        missing_transports.is_empty(),
        "Canonical testbeds 01-05 are missing presentation-binding transports: {missing_transports:?}"
    );
}
