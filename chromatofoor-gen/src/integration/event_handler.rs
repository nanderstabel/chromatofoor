use crate::common::RoutingContext;
use crate::integration::handler_common::{IntegrationHandlerKind, generate_integration_handlers};
use anyhow::Result;
use esdm::EsdmDoc;
use std::path::PathBuf;

pub fn generate_event_handlers(
    out_dir: &PathBuf,
    event_handlers: &[EsdmDoc],
    routing: &RoutingContext,
) -> Result<()> {
    generate_integration_handlers(
        out_dir,
        event_handlers,
        routing,
        IntegrationHandlerKind::EventHandler,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use esdm::*;
    use insta::assert_snapshot;

    #[test]
    fn test_event_handlers_generation() {
        let eh = EsdmDoc {
            name: "TestEventHandler".to_string(),
            kind: "event-handler".to_string(),
            handles: Some(vec![EventRef {
                bounded_context: Some("sales".to_string()),
                aggregate: Some("order".to_string()),
                event: Some("OrderPlaced".to_string()),
            }]),
            ..Default::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = RoutingContext::default();
        generate_event_handlers(&out_dir, &[eh], &routing).unwrap();

        let eh_rs = std::fs::read_to_string(
            out_dir.join("integration/event-handlers/src/generated/handlers/test_event_handler.rs"),
        )
        .unwrap_or_default();

        assert_snapshot!("event_handler_generation", eh_rs);
    }

    #[test]
    fn test_event_handlers_disambiguates_colliding_events_in_trait_and_manual_file() {
        let eh = EsdmDoc {
            name: "AuditNotifier".to_string(),
            kind: "event-handler".to_string(),
            handles: Some(vec![
                EventRef {
                    bounded_context: Some("sales".to_string()),
                    aggregate: Some("order".to_string()),
                    event: Some("Created".to_string()),
                },
                EventRef {
                    bounded_context: Some("sales".to_string()),
                    aggregate: Some("order".to_string()),
                    event: Some("Created".to_string()),
                },
                EventRef {
                    bounded_context: Some("billing".to_string()),
                    aggregate: Some("invoice".to_string()),
                    event: Some("Created".to_string()),
                },
            ]),
            ..Default::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = RoutingContext::default();
        generate_event_handlers(&out_dir, &[eh], &routing).unwrap();

        let gen_rs = std::fs::read_to_string(
            out_dir.join("integration/event-handlers/src/generated/handlers/audit_notifier.rs"),
        )
        .unwrap();
        let manual_rs = std::fs::read_to_string(
            out_dir.join("integration/event-handlers/src/handlers/audit_notifier.rs"),
        )
        .unwrap();

        let normalized_gen: String = gen_rs.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized_gen
                .matches("pub use sales::generated::domain::order::event::Created as SalesOrderCreated;")
                .count(),
            1
        );
        assert!(normalized_gen.contains(
            "pub use billing::generated::domain::invoice::event::Created as BillingInvoiceCreated;"
        ));
        let normalized_manual: String = manual_rs.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized_manual.contains("impl Handle<SalesOrderCreated> for AuditNotifier")
        );
        assert!(
            normalized_manual.contains("impl Handle<BillingInvoiceCreated> for AuditNotifier")
        );
    }
}
