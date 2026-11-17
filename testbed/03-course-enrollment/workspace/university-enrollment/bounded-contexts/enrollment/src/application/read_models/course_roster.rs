#[allow(unused_imports)]
use crate::generated::application::read_models::course_roster::{CourseRoster, CourseRosterRecord};
use crate::generated::domain::course::event::CourseCapacityUpdated;
use crate::generated::domain::course::event::CoursePublished;
use crate::generated::domain::student_course_enrollment::event::EnrollmentDropped;
use crate::generated::domain::student_course_enrollment::event::StudentEnrolled;
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `course-roster` — Materialized course roster tracking capacity and active enrolled count per course.
/// Projection: `course-published`
///
/// Rule: Insert course roster entry keyed by course-id with enrolled-count 0.
impl Project<CoursePublished> for CourseRoster {
    async fn project(&self, event: Envelope<CoursePublished>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.course_id.as_str(), &event, |record| {
                record.course_id = event.data.course_id.to_string();
                record.title.clone_from(&event.data.title);
                record.capacity = event.data.capacity;
                record.enrolled_count = 0;
            })?;
        Ok(())
    }
}
/// Read Model: `course-roster` — Materialized course roster tracking capacity and active enrolled count per course.
/// Projection: `course-capacity-updated`
///
/// Rule: Update capacity for the record keyed by course-id.
impl Project<CourseCapacityUpdated> for CourseRoster {
    async fn project(&self, event: Envelope<CourseCapacityUpdated>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.course_id.as_str(), &event, |record| {
                record.capacity = event.data.capacity;
            })?;
        Ok(())
    }
}
/// Read Model: `course-roster` — Materialized course roster tracking capacity and active enrolled count per course.
/// Projection: `student-enrolled`
///
/// Rule: Increment enrolled-count for the record keyed by course-id.
impl Project<StudentEnrolled> for CourseRoster {
    async fn project(&self, event: Envelope<StudentEnrolled>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.course_id.as_str(), &event, |record| {
                record.enrolled_count += 1;
            })?;
        Ok(())
    }
}
/// Read Model: `course-roster` — Materialized course roster tracking capacity and active enrolled count per course.
/// Projection: `enrollment-dropped`
///
/// Rule: Decrement enrolled-count for the record keyed by course-id.
impl Project<EnrollmentDropped> for CourseRoster {
    async fn project(&self, event: Envelope<EnrollmentDropped>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.course_id.as_str(), &event, |record| {
                record.enrolled_count = (record.enrolled_count - 1).max(0);
            })?;
        Ok(())
    }
}
