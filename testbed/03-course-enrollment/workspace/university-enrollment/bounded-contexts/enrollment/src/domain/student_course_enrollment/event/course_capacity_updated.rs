use crate::generated::domain::course::event::course_capacity_updated::CourseCapacityUpdated;
use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use chromatofoor::bounded_context::Event;
impl Event<StudentCourseEnrollment> for CourseCapacityUpdated {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for CourseCapacityUpdated (return _state if stateless)"
        )
    }
}
