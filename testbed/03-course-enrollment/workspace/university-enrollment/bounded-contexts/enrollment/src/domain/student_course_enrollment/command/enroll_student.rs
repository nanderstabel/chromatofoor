use crate::generated::domain::student_course_enrollment::{
    aggregate::StudentCourseEnrollment, command::EnrollStudent,
    error::StudentCourseEnrollmentError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `enroll-student` — Enrolls a registered student in a published course when seat capacity and course-load limits allow.
///
/// - Actors: `student`, `registrar`
/// - Publishes: `student-enrolled`
/// - Invariants (`StudentCourseEnrollmentError`):
///   - `course-not-published` — Target course must be published before students can enroll in it.
///   - `student-not-registered` — Student must be registered before enrolling in any course.
///   - `course-capacity-exceeded` — Active enrolled students for the target course cannot exceed its current capacity.
///   - `student-load-exceeded` — A student cannot hold more than 5 active course enrollments simultaneously.
impl Command<StudentCourseEnrollment> for EnrollStudent {
    type Error = StudentCourseEnrollmentError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(
        self,
        state: &StudentCourseEnrollment,
    ) -> Result<Vec<StudentCourseEnrollmentEvent>, Self::Error> {
        let student_id = self.student_id.to_string();
        let course_id = self.course_id.to_string();
        if !state.registered_students.contains(&student_id) {
            return Err(StudentCourseEnrollmentError::StudentNotRegistered);
        }
        let Some(&capacity) = state.course_capacities.get(&course_id) else {
            return Err(StudentCourseEnrollmentError::CourseNotPublished);
        };
        if state
            .active_enrollments
            .contains(&(student_id.clone(), course_id.clone()))
        {
            return Ok(vec![]);
        }
        let course_enrolled = i64::try_from(
            state
                .active_enrollments
                .iter()
                .filter(|(_, c)| c == &course_id)
                .count(),
        )
        .unwrap_or(i64::MAX);
        if course_enrolled >= capacity {
            return Err(StudentCourseEnrollmentError::CourseCapacityExceeded);
        }
        let student_load = state
            .active_enrollments
            .iter()
            .filter(|(s, _)| s == &student_id)
            .count();
        if student_load >= 5 {
            return Err(StudentCourseEnrollmentError::StudentLoadExceeded);
        }
        Ok(vec![StudentCourseEnrollmentEvent::StudentEnrolled(
            StudentEnrolled {
                course_id: self.course_id,
                student_id: self.student_id,
            },
        )])
    }
}
