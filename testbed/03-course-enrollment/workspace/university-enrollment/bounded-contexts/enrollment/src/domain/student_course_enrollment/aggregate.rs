use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
/// Dynamic Consistency Boundary: `student-course-enrollment` — Ad-hoc consistency boundary spanning course capacity and active student course load per enrollment decision.
///
/// - Consults:
///   - `course-published` — Course publication event matching the target course-id.
///   - `course-capacity-updated` — Capacity updates matching the target course-id.
///   - `student-registered` — Student registration event matching the target student-id.
///   - `student-enrolled` — All enrollments matching either the target course-id (for seat count) or target student-id (for course load).
///   - `enrollment-dropped` — All dropped enrollments matching either the target course-id or target student-id.
#[derive(Default, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct StudentCourseEnrollmentState {
    pub course_capacities: BTreeMap<String, i64>,
    pub registered_students: BTreeSet<String>,
    pub active_enrollments: BTreeSet<(String, String)>,
}
impl StudentCourseEnrollment {}
