use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use serde::{Deserialize, Serialize};
#[derive(Default, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct StudentCourseEnrollmentState {}
impl StudentCourseEnrollment {}
