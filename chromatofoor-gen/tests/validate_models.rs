use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[test]
fn all_esdm_documents_have_descriptions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let model_paths = vec![
        root.join("testbed/01-bank-account/model.esdm.yaml"),
        root.join("testbed/02-ecommerce-checkout/model.esdm.yaml"),
        root.join("testbed/03-flight-booking/model.esdm.yaml"),
        root.join("testbed/04-virtual-apartment/model.esdm.yaml"),
        root.join("testbed/04-virtual-apartment/model.feature.esdm.yaml"),
        root.join("testbed/05-helpdesk-ticketing/model.esdm.yaml"),
        root.join("testbed/06-notification-delivery/model.esdm.yaml"),
        root.join("testbed/07-life-chart/model.esdm.yaml"),
        root.join("testbed/08-digital-identity-ecosystem/model.esdm.yaml"),
        root.join("testbed/08-digital-identity-ecosystem/model.story.esdm.yaml"),
        root.join("testbed/08-digital-identity-ecosystem/model.feature.esdm.yaml"),
        root.join("testbed/08-digital-identity-ecosystem/model.infra.esdm.yaml"),
    ];

    let mut missing = Vec::new();

    for path in model_paths {
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path).expect("failed to read model file");
        let docs = serde_yaml::Deserializer::from_str(&content);

        for (idx, doc_res) in docs.enumerate() {
            let val: serde_yaml::Value = match serde_yaml::Value::deserialize(doc_res) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let kind = val
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("unknown");
            let name = val
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let description = val.get("description").and_then(|d| d.as_str());

            if description.map_or(true, |d| d.trim().is_empty()) {
                missing.push(format!(
                    "{} -> doc #{}: kind={}, name={}",
                    path.display(),
                    idx + 1,
                    kind,
                    name
                ));
            }
        }
    }

    if !missing.is_empty() {
        panic!(
            "Found {} ESDM documents missing a description:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }
}

#[test]
fn all_esdm_models_pass_extension_and_identity_lint() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let model_dirs: Vec<(&str, Vec<PathBuf>)> = vec![
        (
            "testbed/01-bank-account",
            vec![
                root.join("testbed/01-bank-account/model.esdm.yaml"),
                root.join("testbed/01-bank-account/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/02-ecommerce-checkout",
            vec![
                root.join("testbed/02-ecommerce-checkout/model.esdm.yaml"),
                root.join("testbed/02-ecommerce-checkout/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/03-flight-booking",
            vec![
                root.join("testbed/03-flight-booking/model.esdm.yaml"),
                root.join("testbed/03-flight-booking/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/04-virtual-apartment",
            vec![
                root.join("testbed/04-virtual-apartment/model.esdm.yaml"),
                root.join("testbed/04-virtual-apartment/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/05-helpdesk-ticketing",
            vec![
                root.join("testbed/05-helpdesk-ticketing/model.esdm.yaml"),
                root.join("testbed/05-helpdesk-ticketing/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/06-notification-delivery",
            vec![
                root.join("testbed/06-notification-delivery/model.esdm.yaml"),
                root.join("testbed/06-notification-delivery/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/07-life-chart",
            vec![
                root.join("testbed/07-life-chart/model.esdm.yaml"),
                root.join("testbed/07-life-chart/model.lifecycle.esdm.yaml"),
            ],
        ),
        (
            "testbed/08-digital-identity-ecosystem",
            vec![
                root.join("testbed/08-digital-identity-ecosystem/model.esdm.yaml"),
                root.join("testbed/08-digital-identity-ecosystem/model.lifecycle.esdm.yaml"),
            ],
        ),
    ];

    let mut all_failures = Vec::new();

    for (model_name, files) in model_dirs {
        let mut docs = Vec::new();
        for path in files {
            if !path.exists() {
                continue;
            }
            let content = fs::read_to_string(&path).expect("failed to read model file");
            for doc_res in serde_yaml::Deserializer::from_str(&content) {
                if let Ok(doc) = esdm::EsdmDoc::deserialize(doc_res) {
                    docs.push(doc);
                }
            }
        }
        let violations = esdm::lint::validate_model(&docs);
        for v in violations {
            all_failures.push(format!("{model_name}: {v}"));
        }
    }

    if !all_failures.is_empty() {
        panic!(
            "Found {} ESDM model lint violations:\n{}",
            all_failures.len(),
            all_failures.join("\n")
        );
    }
}
