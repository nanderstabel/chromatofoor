use crate::generator::event_handlers::{IntegrationHandlerKind, generate_integration_handlers};
use crate::model::EsdmDoc;
use anyhow::Result;
use std::path::PathBuf;

pub fn generate_policies(
    out_dir: &PathBuf,
    policies: &[EsdmDoc],
    routing: &crate::generator::RoutingContext,
) -> Result<()> {
    generate_integration_handlers(out_dir, policies, routing, IntegrationHandlerKind::Policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use insta::assert_snapshot;

    #[test]
    fn test_policies_generation() {
        let eh = EsdmDoc {
            name: "TestEventHandler".to_string(),
            kind: "policy".to_string(),
            handles: Some(vec![EventRef {
                bounded_context: Some("sales".to_string()),
                aggregate: Some("order".to_string()),
                event: Some("OrderPlaced".to_string()),
            }]),
            emits: Some(vec![CommandRef {
                bounded_context: Some("shipping".to_string()),
                aggregate: Some("shipment".to_string()),
                command: Some("ScheduleShipment".to_string()),
            }]),
            ..Default::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = crate::generator::RoutingContext::default();
        generate_policies(&out_dir, &[eh], &routing).unwrap();

        let eh_rs = std::fs::read_to_string(
            out_dir.join("integration/policies/src/generated/handlers/test_event_handler.rs"),
        )
        .unwrap_or_default();

        assert_snapshot!("policy_handler_generation", eh_rs);
    }
}
