use crate::{DataSchema, EsdmDoc};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintViolation {
    pub rule_id: &'static str,
    pub kind: String,
    pub name: String,
    pub bounded_context: Option<String>,
    pub message: String,
}

impl fmt::Display for LintViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(bc) = &self.bounded_context {
            write!(
                f,
                "[{}] {} '{}/{}': {}",
                self.rule_id, self.kind, bc, self.name, self.message
            )
        } else {
            write!(
                f,
                "[{}] {} '{}': {}",
                self.rule_id, self.kind, self.name, self.message
            )
        }
    }
}

/// Validates cross-document ESDM model and extension rules (`lifecycle/v1` and
/// strict identifier field presence and value types across aggregates, DCBs,
/// entities, and commands).
#[must_use]
pub fn validate_model(docs: &[EsdmDoc]) -> Vec<LintViolation> {
    let mut violations = Vec::new();

    let mut aggregates: BTreeMap<(String, String), &EsdmDoc> = BTreeMap::new();
    let mut dcbs: BTreeMap<(String, String), &EsdmDoc> = BTreeMap::new();
    let mut lifecycles: BTreeMap<(String, String), &EsdmDoc> = BTreeMap::new();
    let mut lifecycle_docs: Vec<(String, String, &EsdmDoc)> = Vec::new();
    let mut agg_commands: BTreeMap<(String, String), Vec<&EsdmDoc>> = BTreeMap::new();
    let mut dcb_commands: BTreeMap<(String, String), Vec<&EsdmDoc>> = BTreeMap::new();
    let mut entities: Vec<&EsdmDoc> = Vec::new();

    for doc in docs {
        match doc.kind.as_str() {
            "aggregate" => {
                if let Some(bc) = doc.scope.as_ref().and_then(|s| s.bounded_context.clone()) {
                    aggregates.insert((bc, doc.name.clone()), doc);
                }
            }
            "dynamic-consistency-boundary" => {
                if let Some(bc) = doc.scope.as_ref().and_then(|s| s.bounded_context.clone()) {
                    dcbs.insert((bc, doc.name.clone()), doc);
                }
            }
            "aggregate-lifecycle" => {
                if let Some(scope) = &doc.scope
                    && let (Some(bc), Some(agg)) =
                        (scope.bounded_context.clone(), scope.aggregate.clone())
                {
                    if let Some(prev) = lifecycles.get(&(bc.clone(), agg.clone())) {
                        violations.push(LintViolation {
                            rule_id: "esdm/lifecycle/duplicate-aggregate-lifecycle",
                            kind: "aggregate-lifecycle".to_string(),
                            name: doc.name.clone(),
                            bounded_context: Some(bc.clone()),
                            message: format!(
                                "duplicate aggregate-lifecycle for aggregate '{bc}/{agg}' (already defined by '{}')",
                                prev.name
                            ),
                        });
                    } else {
                        lifecycles.insert((bc.clone(), agg.clone()), doc);
                    }
                    lifecycle_docs.push((bc, agg, doc));
                } else {
                    violations.push(LintViolation {
                        rule_id: "esdm/lifecycle/missing-scope",
                        kind: "aggregate-lifecycle".to_string(),
                        name: doc.name.clone(),
                        bounded_context: doc
                            .scope
                            .as_ref()
                            .and_then(|s| s.bounded_context.clone()),
                        message: "aggregate-lifecycle must specify both 'scope.bounded-context' and 'scope.aggregate'".to_string(),
                    });
                }
            }
            "command" => {
                if let Some(scope) = &doc.scope
                    && let Some(bc) = scope.bounded_context.clone()
                {
                    if let Some(agg) = scope.aggregate.clone() {
                        agg_commands.entry((bc, agg)).or_default().push(doc);
                    } else if let Some(dcb) = scope.dynamic_consistency_boundary.clone() {
                        dcb_commands.entry((bc, dcb)).or_default().push(doc);
                    }
                }
            }
            "entity" => {
                entities.push(doc);
            }
            _ => {}
        }
    }

    // 1. Validate aggregate-lifecycle documents against aggregates and their commands
    for (bc, agg, lc) in &lifecycle_docs {
        if !aggregates.contains_key(&(bc.clone(), agg.clone())) {
            violations.push(LintViolation {
                rule_id: "esdm/lifecycle/unknown-aggregate",
                kind: "aggregate-lifecycle".to_string(),
                name: lc.name.clone(),
                bounded_context: Some(bc.clone()),
                message: format!("references non-existent aggregate '{bc}/{agg}'"),
            });
            continue;
        }

        let cmds = agg_commands
            .get(&(bc.clone(), agg.clone()))
            .map_or(&[][..], Vec::as_slice);
        let created = lc.created.as_deref().unwrap_or_default();
        if created.is_empty() {
            violations.push(LintViolation {
                rule_id: "esdm/lifecycle/empty-created",
                kind: "aggregate-lifecycle".to_string(),
                name: lc.name.clone(),
                bounded_context: Some(bc.clone()),
                message: format!(
                    "for aggregate '{bc}/{agg}' must list at least one 'created' command"
                ),
            });
        }
        for cmd_name in created {
            if !cmds.iter().any(|c| &c.name == cmd_name) {
                violations.push(LintViolation {
                    rule_id: "esdm/lifecycle/unknown-created-command",
                    kind: "aggregate-lifecycle".to_string(),
                    name: lc.name.clone(),
                    bounded_context: Some(bc.clone()),
                    message: format!(
                        "lists unknown created command '{cmd_name}' for aggregate '{bc}/{agg}'"
                    ),
                });
            }
        }
        if let Some(mutated) = &lc.mutated {
            for cmd_name in mutated {
                if !cmds.iter().any(|c| &c.name == cmd_name) {
                    violations.push(LintViolation {
                        rule_id: "esdm/lifecycle/unknown-mutated-command",
                        kind: "aggregate-lifecycle".to_string(),
                        name: lc.name.clone(),
                        bounded_context: Some(bc.clone()),
                        message: format!(
                            "lists unknown mutated command '{cmd_name}' for aggregate '{bc}/{agg}'"
                        ),
                    });
                }
                if created.contains(cmd_name) {
                    violations.push(LintViolation {
                        rule_id: "esdm/lifecycle/conflicting-command",
                        kind: "aggregate-lifecycle".to_string(),
                        name: lc.name.clone(),
                        bounded_context: Some(bc.clone()),
                        message: format!(
                            "lists command '{cmd_name}' in both 'created' and 'mutated'"
                        ),
                    });
                }
            }
        }
        if let Some(deleted) = &lc.deleted {
            let mutated_slice = lc.mutated.as_deref().unwrap_or_default();
            for cmd_name in deleted {
                if !cmds.iter().any(|c| &c.name == cmd_name) {
                    violations.push(LintViolation {
                        rule_id: "esdm/lifecycle/unknown-deleted-command",
                        kind: "aggregate-lifecycle".to_string(),
                        name: lc.name.clone(),
                        bounded_context: Some(bc.clone()),
                        message: format!(
                            "lists unknown deleted command '{cmd_name}' for aggregate '{bc}/{agg}'"
                        ),
                    });
                }
                if created.contains(cmd_name) {
                    violations.push(LintViolation {
                        rule_id: "esdm/lifecycle/conflicting-command",
                        kind: "aggregate-lifecycle".to_string(),
                        name: lc.name.clone(),
                        bounded_context: Some(bc.clone()),
                        message: format!(
                            "lists command '{cmd_name}' in both 'created' and 'deleted'"
                        ),
                    });
                }
                if mutated_slice.contains(cmd_name) {
                    violations.push(LintViolation {
                        rule_id: "esdm/lifecycle/conflicting-command",
                        kind: "aggregate-lifecycle".to_string(),
                        name: lc.name.clone(),
                        bounded_context: Some(bc.clone()),
                        message: format!(
                            "lists command '{cmd_name}' in both 'mutated' and 'deleted'"
                        ),
                    });
                }
            }
        }
    }

    // 2. Validate aggregates and their commands
    for ((bc, agg), agg_doc) in &aggregates {
        let cmds = agg_commands
            .get(&(bc.clone(), agg.clone()))
            .map_or(&[][..], Vec::as_slice);
        let lc = lifecycles.get(&(bc.clone(), agg.clone())).copied();
        let created_set: BTreeSet<&str> = lc
            .and_then(|l| l.created.as_deref())
            .unwrap_or_default()
            .iter()
            .map(String::as_str)
            .collect();

        let requires_lifecycle = agg_doc.identified_by.as_ref().is_none_or(|spec| {
            let slice = spec.as_slice();
            slice.is_empty()
                || slice
                    .iter()
                    .any(|e| matches!(e.source.as_str(), "state" | "generated"))
        });

        if requires_lifecycle && !cmds.is_empty() && created_set.is_empty() {
            violations.push(LintViolation {
                rule_id: "esdm/lifecycle/missing-aggregate-lifecycle",
                kind: "aggregate".to_string(),
                name: agg.clone(),
                bounded_context: Some(bc.clone()),
                message: format!(
                    "Aggregate '{bc}/{agg}' has state/generated identity and commands, but is missing a 'kind: aggregate-lifecycle' definition with at least one 'created' command in model.lifecycle.esdm.yaml"
                ),
            });
        }

        if let Some(spec) = &agg_doc.identified_by {
            for entry in spec.as_slice() {
                match entry.source.as_str() {
                    "state" => {
                        let field = entry.field.as_deref().unwrap_or("id");
                        let state_format = check_id_field(
                            agg_doc.state.as_ref(),
                            field,
                            None,
                            "esdm/identity/aggregate-state-id-field",
                            "aggregate",
                            agg,
                            Some(bc),
                            "state",
                            &mut violations,
                        );
                        for cmd in cmds {
                            check_id_field(
                                cmd.data.as_ref(),
                                field,
                                state_format.as_deref(),
                                "esdm/identity/command-missing-state-id-field",
                                "command",
                                &cmd.name,
                                Some(bc),
                                &format!(
                                    "data (required by aggregate '{agg}' identifiedBy.state '{field}')"
                                ),
                                &mut violations,
                            );
                        }
                    }
                    "generated" => {
                        let generator = entry.generator.as_deref().unwrap_or("uuid");
                        let expected_format = if generator == "uuid" {
                            Some("uuid")
                        } else {
                            Some("identifier")
                        };
                        for cmd in cmds {
                            let is_create = created_set.contains(cmd.name.as_str());
                            if !is_create {
                                check_id_field(
                                    cmd.data.as_ref(),
                                    "id",
                                    expected_format,
                                    "esdm/identity/command-missing-generated-id-field",
                                    "command",
                                    &cmd.name,
                                    Some(bc),
                                    &format!(
                                        "data (mutation/deletion command on aggregate '{agg}' with generated({generator}) identity)"
                                    ),
                                    &mut violations,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 3. Validate DCBs and their commands
    for ((bc, dcb), dcb_doc) in &dcbs {
        let cmds = dcb_commands
            .get(&(bc.clone(), dcb.clone()))
            .map_or(&[][..], Vec::as_slice);
        if let Some(spec) = &dcb_doc.identified_by {
            for entry in spec.as_slice() {
                if entry.source == "command-payload" {
                    let field = entry.field.as_deref().unwrap_or("id");
                    for cmd in cmds {
                        check_id_field(
                            cmd.data.as_ref(),
                            field,
                            None,
                            "esdm/identity/dcb-command-id-field",
                            "command",
                            &cmd.name,
                            Some(bc),
                            &format!(
                                "data (required by dynamic-consistency-boundary '{dcb}' identifiedBy.command-payload '{field}')"
                            ),
                            &mut violations,
                        );
                    }
                }
            }
        }
    }

    // 4. Validate entities
    for entity in entities {
        let bc = entity
            .scope
            .as_ref()
            .and_then(|s| s.bounded_context.as_deref());
        if let Some(spec) = &entity.identified_by {
            for entry in spec.as_slice() {
                if entry.source == "schema" {
                    let field = entry.field.as_deref().unwrap_or("id");
                    check_id_field(
                        entity.schema.as_ref(),
                        field,
                        None,
                        "esdm/identity/entity-schema-id-field",
                        "entity",
                        &entity.name,
                        bc,
                        "schema",
                        &mut violations,
                    );
                }
            }
        }
    }

    violations
}

#[allow(clippy::too_many_arguments)]
fn check_id_field(
    schema: Option<&DataSchema>,
    field: &str,
    expected_format: Option<&str>,
    rule_id: &'static str,
    kind: &str,
    name: &str,
    bc: Option<&str>,
    schema_location: &str,
    violations: &mut Vec<LintViolation>,
) -> Option<String> {
    let Some(schema) = schema else {
        violations.push(LintViolation {
            rule_id,
            kind: kind.to_string(),
            name: name.to_string(),
            bounded_context: bc.map(ToOwned::to_owned),
            message: format!(
                "missing identifier field '{field}' in {schema_location} (schema is empty)"
            ),
        });
        return None;
    };

    let Some(prop) = schema.properties.get(field) else {
        violations.push(LintViolation {
            rule_id,
            kind: kind.to_string(),
            name: name.to_string(),
            bounded_context: bc.map(ToOwned::to_owned),
            message: format!("missing identifier field '{field}' in {schema_location}.properties"),
        });
        return None;
    };

    if !schema.required.iter().any(|r| r == field) {
        violations.push(LintViolation {
            rule_id,
            kind: kind.to_string(),
            name: name.to_string(),
            bounded_context: bc.map(ToOwned::to_owned),
            message: format!(
                "identifier field '{field}' in {schema_location} must be listed in 'required'"
            ),
        });
    }

    let prop_type = prop.prop_type.as_deref().unwrap_or("");
    let prop_format = prop.format.as_deref();

    if prop_type != "string" {
        violations.push(LintViolation {
            rule_id,
            kind: kind.to_string(),
            name: name.to_string(),
            bounded_context: bc.map(ToOwned::to_owned),
            message: format!(
                "identifier field '{field}' in {schema_location} has type '{prop_type}', expected 'type: string' with 'format: uuid' or 'format: identifier'"
            ),
        });
        return None;
    }

    if let Some(exp_fmt) = expected_format {
        if prop_format != Some(exp_fmt) {
            violations.push(LintViolation {
                rule_id,
                kind: kind.to_string(),
                name: name.to_string(),
                bounded_context: bc.map(ToOwned::to_owned),
                message: format!(
                    "identifier field '{field}' in {schema_location} has format {prop_format:?}, expected 'format: {exp_fmt}'"
                ),
            });
            return None;
        }
        return Some(exp_fmt.to_string());
    }

    match prop_format {
        Some("uuid") => Some("uuid".to_string()),
        Some("identifier") => Some("identifier".to_string()),
        other => {
            violations.push(LintViolation {
                rule_id,
                kind: kind.to_string(),
                name: name.to_string(),
                bounded_context: bc.map(ToOwned::to_owned),
                message: format!(
                    "identifier field '{field}' in {schema_location} has format {other:?}, expected 'format: uuid' or 'format: identifier'"
                ),
            });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scope;

    #[test]
    fn detects_duplicate_and_missing_scope_aggregate_lifecycle() {
        let agg = EsdmDoc {
            kind: "aggregate".to_string(),
            name: "order".to_string(),
            scope: Some(Scope {
                bounded_context: Some("sales".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let cmd = EsdmDoc {
            kind: "command".to_string(),
            name: "place-order".to_string(),
            scope: Some(Scope {
                bounded_context: Some("sales".to_string()),
                aggregate: Some("order".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let lc1 = EsdmDoc {
            kind: "aggregate-lifecycle".to_string(),
            name: "order-lifecycle-1".to_string(),
            scope: Some(Scope {
                bounded_context: Some("sales".to_string()),
                aggregate: Some("order".to_string()),
                ..Default::default()
            }),
            created: Some(vec!["place-order".to_string()]),
            mutated: Some(vec!["ghost-mutate".to_string(), "place-order".to_string()]),
            deleted: Some(vec!["ghost-delete".to_string()]),
            ..Default::default()
        };
        let lc2 = EsdmDoc {
            kind: "aggregate-lifecycle".to_string(),
            name: "order-lifecycle-2".to_string(),
            scope: Some(Scope {
                bounded_context: Some("sales".to_string()),
                aggregate: Some("order".to_string()),
                ..Default::default()
            }),
            created: Some(vec!["place-order".to_string()]),
            ..Default::default()
        };
        let lc_missing_scope = EsdmDoc {
            kind: "aggregate-lifecycle".to_string(),
            name: "orphan-lifecycle".to_string(),
            scope: Some(Scope {
                bounded_context: Some("sales".to_string()),
                aggregate: None,
                ..Default::default()
            }),
            created: Some(vec!["place-order".to_string()]),
            ..Default::default()
        };

        let violations = validate_model(&[agg, cmd, lc1, lc2, lc_missing_scope]);
        assert!(
            violations.iter().any(
                |v| v.rule_id == "esdm/lifecycle/duplicate-aggregate-lifecycle"
                    && v.name == "order-lifecycle-2"
            ),
            "Expected duplicate-aggregate-lifecycle violation, got: {violations:?}"
        );
        assert!(
            violations.iter().any(
                |v| v.rule_id == "esdm/lifecycle/missing-scope" && v.name == "orphan-lifecycle"
            ),
            "Expected missing-scope violation, got: {violations:?}"
        );
        assert!(
            violations
                .iter()
                .any(|v| v.rule_id == "esdm/lifecycle/unknown-mutated-command"
                    && v.message.contains("ghost-mutate")),
            "Expected unknown-mutated-command violation, got: {violations:?}"
        );
        assert!(
            violations
                .iter()
                .any(|v| v.rule_id == "esdm/lifecycle/unknown-deleted-command"
                    && v.message.contains("ghost-delete")),
            "Expected unknown-deleted-command violation, got: {violations:?}"
        );
        assert!(
            violations
                .iter()
                .any(|v| v.rule_id == "esdm/lifecycle/conflicting-command"
                    && v.message.contains("place-order")),
            "Expected conflicting-command violation, got: {violations:?}"
        );
    }
}
