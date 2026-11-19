extern crate self as chromatofoor;

pub mod bounded_context;
pub mod common;
pub mod domain;
pub mod integration;
pub mod runtime;
pub mod testing;

pub use architecturekit;
pub use chromatofoor_derive::{
    Events, Schema, actor, aggregate, bounded_context, command, context_mapping, domain,
    domain_service, dynamic_consistency_boundary, entity, event, event_handler, external_system,
    policy, process_manager, query, read_model, subdomain, value_object,
};
pub use chrono;
pub use common::{Identifier, Problem, ProblemDetails, SubjectSegment, SubjectSegmentError};
pub use constcat;
pub use serde;
pub use serde_json;
pub use tracing;
pub use uuid;
pub use validator;

/// Resolves an instance [`architecturekit::Subject`] under `<A as common::Kind>::TOPIC`.
#[must_use]
pub fn subject_for<A: common::Kind>(id: impl AsRef<str>) -> architecturekit::Subject {
    architecturekit::Subject::new(&format!("{}/{}", A::TOPIC, id.as_ref()))
}
