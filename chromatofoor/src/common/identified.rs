use crate::common::Kind;
use nutype::nutype;

fn is_valid_subject_segment(segment: &str) -> bool {
    segment
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A validated, non-empty [`architecturekit::Subject`] path segment containing only
/// ASCII letters, digits, underscores (`_`), and hyphens (`-`).
///
/// ```
/// use chromatofoor::common::SubjectSegment;
///
/// let seg = SubjectSegment::try_new("ord-42").unwrap();
/// assert_eq!(seg.into_inner(), "ord-42");
///
/// // Empty strings and characters outside `[A-Za-z0-9_-]` are rejected:
/// assert!(SubjectSegment::try_new("").is_err());
/// assert!(SubjectSegment::try_new("ord/42").is_err());
/// assert!(SubjectSegment::try_new("user@example.com").is_err());
/// ```
#[nutype(
    validate(not_empty, predicate = is_valid_subject_segment),
    derive(
        Clone,
        Debug,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        AsRef,
        Deref,
        Borrow,
        Display,
        FromStr,
        TryFrom,
        Into,
        Serialize,
        Deserialize,
    )
)]
pub struct SubjectSegment(String);

impl architecturekit::Schema for SubjectSegment {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "string",
            "pattern": "^[A-Za-z0-9_-]+$"
        })
    }
}

/// A domain identifier backed by a validated [`SubjectSegment`].
///
/// Deserializing or parsing an `Identifier` via `serde` or [`std::str::FromStr`] validates
/// the segment and returns an error if it is empty or contains characters outside
/// `[A-Za-z0-9_-]`. For ergonomic construction in Rust code, `From<&str>`, `From<String>`,
/// and `From<uuid::Uuid>` are also provided (panicking if the given string is not a valid
/// [`SubjectSegment`]).
///
/// ```
/// use chromatofoor::common::Identifier;
///
/// let id: Identifier = "ord-42".into();
/// assert_eq!(id.as_str(), "ord-42");
///
/// let parsed: Identifier = "cust_99".parse().unwrap();
/// assert_eq!(parsed.as_str(), "cust_99");
///
/// // Invalid segments are rejected when parsing or deserializing:
/// assert!("".parse::<Identifier>().is_err());
/// assert!("ord/42".parse::<Identifier>().is_err());
/// assert!(serde_json::from_str::<Identifier>("\"\"").is_err());
/// ```
#[nutype(derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRef,
    Deref,
    Borrow,
    Display,
    From,
    Into,
    Serialize,
    Deserialize,
))]
pub struct Identifier(SubjectSegment);

impl Identifier {
    /// Fallibly constructs an [`Identifier`] from a string, returning a
    /// [`SubjectSegmentError`] if the string is empty or contains invalid characters.
    pub fn try_new(raw: impl Into<String>) -> Result<Self, SubjectSegmentError> {
        SubjectSegment::try_new(raw).map(Self::new)
    }

    /// Returns the underlying identifier slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        <Self as std::ops::Deref>::deref(self).as_ref()
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::str::FromStr for Identifier {
    type Err = SubjectSegmentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SubjectSegment::try_new(s).map(Self::new)
    }
}

impl From<&str> for Identifier {
    #[allow(clippy::panic)]
    fn from(s: &str) -> Self {
        let seg =
            SubjectSegment::try_new(s).unwrap_or_else(|e| panic!("invalid Identifier {s:?}: {e}"));
        Self::new(seg)
    }
}

impl From<String> for Identifier {
    #[allow(clippy::panic)]
    fn from(s: String) -> Self {
        let seg = SubjectSegment::try_new(s.as_str())
            .unwrap_or_else(|e| panic!("invalid Identifier {s:?}: {e}"));
        Self::new(seg)
    }
}

impl From<&String> for Identifier {
    fn from(s: &String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<uuid::Uuid> for Identifier {
    fn from(id: uuid::Uuid) -> Self {
        Self::from(id.to_string())
    }
}

impl architecturekit::Schema for Identifier {
    fn schema() -> serde_json::Value {
        <SubjectSegment as architecturekit::Schema>::schema()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdSource {
    State(&'static str),
    CommandPayload(&'static str),
    Schema(&'static str),
    Static(&'static str),
    Generated(&'static str),
}

pub trait Identified {
    const IDENTIFIED_BY: &'static [IdSource];
    const HAS_LIFECYCLE: bool = false;
}

pub trait FieldAsIdStr<'a> {
    fn as_id_str(&'a self) -> Option<std::borrow::Cow<'a, str>>;
}

impl<'a> FieldAsIdStr<'a> for SubjectSegment {
    fn as_id_str(&'a self) -> Option<std::borrow::Cow<'a, str>> {
        Some(std::borrow::Cow::Borrowed(self.as_ref()))
    }
}

impl<'a> FieldAsIdStr<'a> for Identifier {
    fn as_id_str(&'a self) -> Option<std::borrow::Cow<'a, str>> {
        Some(std::borrow::Cow::Borrowed(self.as_str()))
    }
}

impl<'a> FieldAsIdStr<'a> for uuid::Uuid {
    fn as_id_str(&'a self) -> Option<std::borrow::Cow<'a, str>> {
        Some(std::borrow::Cow::Owned(self.to_string()))
    }
}

impl<'a, T: FieldAsIdStr<'a>> FieldAsIdStr<'a> for Option<T> {
    fn as_id_str(&'a self) -> Option<std::borrow::Cow<'a, str>> {
        self.as_ref()?.as_id_str()
    }
}

pub trait NonIdField<'a> {
    fn as_id_str(&self) -> Option<std::borrow::Cow<'a, str>> {
        None
    }
}

impl<'a, T: ?Sized> NonIdField<'a> for &'a T {}

pub trait ValidIdField {}

impl ValidIdField for SubjectSegment {}
impl ValidIdField for Identifier {}
impl ValidIdField for uuid::Uuid {}

pub struct IdFieldProbe<T: ?Sized>(std::marker::PhantomData<T>);

impl<T: ValidIdField> IdFieldProbe<T> {
    pub const IS_VALID: bool = true;
}

pub trait NonValidIdField {
    const IS_VALID: bool = false;
}

impl<T: ?Sized> NonValidIdField for IdFieldProbe<T> {}

#[must_use]
pub const fn const_str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn contains_valid_id_field(fields: &[(&str, bool)], target: &str) -> bool {
    let mut i = 0;
    while i < fields.len() {
        let (name, is_valid_id_type) = fields[i];
        if const_str_eq(name, target) {
            return is_valid_id_type;
        }
        i += 1;
    }
    false
}

#[must_use]
pub const fn validate_command_id_fields(
    sources: &[IdSource],
    command_fields: &[(&str, bool)],
    is_create: bool,
) -> bool {
    let mut i = 0;
    while i < sources.len() {
        match sources[i] {
            IdSource::State(field) | IdSource::CommandPayload(field) | IdSource::Schema(field) => {
                if !contains_valid_id_field(command_fields, field) {
                    return false;
                }
            }
            IdSource::Static(_) => {}
            IdSource::Generated(_) => {
                if is_create {
                    let mut j = 0;
                    while j < command_fields.len() {
                        let (name, is_valid_id_type) = command_fields[j];
                        if const_str_eq(name, "id") && !is_valid_id_type {
                            return false;
                        }
                        j += 1;
                    }
                } else if !contains_valid_id_field(command_fields, "id") {
                    return false;
                }
            }
        }
        i += 1;
    }
    true
}

#[must_use]
pub fn resolve_subject<S: Kind + Identified>(
    command_fields: &[(&str, Option<std::borrow::Cow<'_, str>>)],
) -> architecturekit::Subject {
    let mut parts = Vec::with_capacity(S::IDENTIFIED_BY.len());
    for source in S::IDENTIFIED_BY {
        match *source {
            IdSource::State(field) | IdSource::CommandPayload(field) | IdSource::Schema(field) => {
                let val = command_fields
                    .iter()
                    .find_map(|(k, v)| (*k == field).then_some(v.as_deref()).flatten())
                    .unwrap_or_default();
                parts.push(val.to_string());
            }
            IdSource::Static(val) => {
                parts.push(val.to_string());
            }
            IdSource::Generated(_) => {
                if let Some(val) = command_fields
                    .iter()
                    .find_map(|(k, v)| (*k == "id").then_some(v.as_deref()).flatten())
                {
                    parts.push(val.to_string());
                } else {
                    parts.push(uuid::Uuid::new_v4().to_string());
                }
            }
        }
    }
    architecturekit::Subject::new(&format!("{}/{}", S::TOPIC, parts.join("/")))
}
