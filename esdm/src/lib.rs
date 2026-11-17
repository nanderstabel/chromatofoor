use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub mod lint;

#[cfg(feature = "openapi")]
pub mod openapi;

#[cfg(feature = "jsonschema")]
pub mod jsonschema;

#[cfg(feature = "typescript")]
pub mod typescript;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub labels: Option<IndexMap<String, String>>,
    pub annotations: Option<IndexMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EsdmDoc {
    pub kind: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "boundedContexts")]
    pub bounded_contexts: Option<Vec<String>>,
    pub scope: Option<Scope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identified_by: Option<IdentifiedBy>,
    pub data: Option<DataSchema>,
    pub state: Option<DataSchema>,
    pub schema: Option<DataSchema>,
    pub projections: Option<Vec<Projection>>,
    #[serde(rename = "readModel")]
    pub read_model: Option<String>,
    pub result: Option<PropertySchema>,
    pub parameters: Option<DataSchema>,
    pub publishes: Option<Vec<String>>,
    pub actors: Option<Vec<String>>,
    pub consults: Option<Vec<Consult>>,
    pub customer: Option<MappingEnd>,
    pub supplier: Option<MappingEnd>,
    pub conformist: Option<MappingEnd>,
    pub upstream: Option<MappingEnd>,
    pub downstream: Option<MappingEnd>,
    pub host: Option<MappingEnd>,
    pub consumer: Option<MappingEnd>,
    pub publisher: Option<MappingEnd>,
    pub participants: Option<Vec<MappingEnd>>,
    pub correlated_by: Option<CorrelatedBy>,
    pub delivery_guarantee: Option<String>,
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub idempotency: Option<Idempotency>,
    pub starts_when: Option<Vec<EventRef>>,
    pub ends_when: Option<Vec<NamedCondition>>,
    pub timers: Option<Vec<Timer>>,
    pub reactions: Option<Vec<Reaction>>,
    pub scenarios: Option<Vec<Scenario>>,
    pub handles: Option<Vec<EventRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effects: Option<Vec<SideEffect>>,
    pub emits: Option<Vec<CommandRef>>,
    pub invariants: Option<Vec<Invariant>>,
    pub constraints: Option<Vec<Invariant>>,
    pub functions: Option<Vec<DomainFunction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<IndexMap<String, serde_yaml::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest: Option<RestBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli: Option<CliBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphql: Option<GraphqlBinding>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "tauriIpc")]
    pub tauri_ipc: Option<TauriIpcBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutated: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestBinding {
    pub path: String,
    pub method: String,
    pub success_status: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CliBinding {
    pub command: String,
    pub subcommand: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphqlBinding {
    pub operation_type: String,
    pub field: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TauriIpcBinding {
    pub command_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DomainFunction {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: DataSchema,
    #[serde(default)]
    pub returns: DataSchema,
    pub rules: Option<Vec<Invariant>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Invariant {
    pub name: String,
    pub rule: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub given: Vec<GivenEvent>,
    pub when: WhenAction,
    pub then: ThenExpectation,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GivenEvent {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub event: String,
    #[serde(default)]
    pub data: serde_yaml::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WhenAction {
    pub event: Option<String>,
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub timer: Option<String>,
    pub command: Option<String>,
    pub query: Option<String>,
    #[serde(default)]
    pub data: serde_yaml::Value,
    #[serde(default)]
    pub parameters: serde_yaml::Value,
    pub actor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedCommand {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub command: String,
    #[serde(default)]
    pub data: serde_yaml::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ThenExpectation {
    pub events: Option<Vec<GivenEvent>>,
    pub commands: Option<Vec<ExpectedCommand>>,
    pub emits: Option<Vec<ExpectedCommand>>,
    #[serde(rename = "setTimers")]
    pub set_timers: Option<Vec<String>>,
    #[serde(rename = "cancelTimers")]
    pub cancel_timers: Option<Vec<String>>,
    pub state: Option<serde_yaml::Value>,
    pub ended: Option<bool>,
    pub rejection: Option<RejectionExpectation>,
    pub result: Option<serde_yaml::Value>,
    #[serde(rename = "readModel")]
    pub read_model: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RejectionExpectation {
    pub invariant: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Projection {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub event: Option<String>,
    pub rule: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub domain: Option<String>,
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    #[serde(rename = "dynamicConsistencyBoundary")]
    pub dynamic_consistency_boundary: Option<String>,
    #[serde(rename = "readModel")]
    pub read_model: Option<String>,
    #[serde(rename = "processManager")]
    pub process_manager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Consult {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub event: Option<String>,
    pub criteria: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MappingEnd {
    pub domain: Option<String>,
    pub bounded_context: Option<String>,
    pub external_system: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventRef {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub event: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReactionWhen {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub event: Option<String>,
    pub timer: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    pub when: Option<ReactionWhen>,
    pub rule: Option<String>,
    pub emits: Option<Vec<CommandRef>>,
    #[serde(rename = "setTimers")]
    pub set_timers: Option<Vec<String>>,
    #[serde(rename = "cancelTimers")]
    pub cancel_timers: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommandRef {
    pub bounded_context: Option<String>,
    pub aggregate: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct DataSchema {
    #[serde(default)]
    pub properties: IndexMap<String, PropertySchema>,
    #[serde(default)]
    pub required: Vec<String>,
    pub example: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PropertySchema {
    #[serde(rename = "$ref")]
    pub ref_path: Option<String>,
    #[serde(rename = "type")]
    pub prop_type: Option<String>,
    pub format: Option<String>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    pub items: Option<Box<PropertySchema>>,
    pub properties: Option<IndexMap<String, PropertySchema>>,
    pub required: Option<Vec<String>>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    #[serde(rename = "minLength")]
    pub min_length: Option<usize>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub example: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IdentifiedByEntry {
    pub name: Option<String>,
    pub source: String,
    pub field: Option<String>,
    pub value: Option<String>,
    pub generator: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum IdentifiedBy {
    Single(IdentifiedByEntry),
    Multiple(Vec<IdentifiedByEntry>),
}

impl IdentifiedBy {
    #[must_use]
    pub fn as_slice(&self) -> &[IdentifiedByEntry] {
        match self {
            Self::Single(entry) => std::slice::from_ref(entry),
            Self::Multiple(entries) => entries.as_slice(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CorrelatedBy {
    pub source: String,
    pub field: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NamedCondition {
    pub name: String,
    pub condition: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerAfter {
    pub value: i64,
    pub unit: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    pub name: String,
    pub description: Option<String>,
    pub after: Option<TimerAfter>,
    pub at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SideEffect {
    #[serde(rename = "type")]
    pub effect_type: Option<String>,
    pub external_system: Option<String>,
    pub rule: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Idempotency {
    pub owner: String,
    pub strategy: Option<String>,
}
