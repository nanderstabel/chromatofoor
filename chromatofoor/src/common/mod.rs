pub mod identified;
pub mod kind;
pub mod problem;
#[doc(hidden)]
pub mod schema;
pub mod scope;
pub mod topic;

pub use identified::{
    FieldAsIdStr, IdFieldProbe, IdSource, Identified, Identifier, NonIdField, NonValidIdField,
    SubjectSegment, SubjectSegmentError, ValidIdField, const_str_eq, resolve_subject,
    validate_command_id_fields,
};
pub use kind::Kind;
pub use problem::{Problem, ProblemDetails};
pub use scope::Scope;
pub use topic::{Topic, topic_in_domain};
