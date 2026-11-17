#[allow(unused_imports)]
use crate::generated::application::read_models::course_roster::{CourseRoster, CourseRosterRecord};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::course::event::course_published::CoursePublished>
    for CourseRoster
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::course::event::course_published::CoursePublished,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CoursePublished::project for CourseRoster")
    }
}
impl
    Project<crate::generated::domain::course::event::course_capacity_updated::CourseCapacityUpdated>
    for CourseRoster
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::course::event::course_capacity_updated::CourseCapacityUpdated,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CourseCapacityUpdated::project for CourseRoster")
    }
}
impl Project<
    crate::generated::domain::student_course_enrollment::event::student_enrolled::StudentEnrolled,
> for CourseRoster {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::student_course_enrollment::event::student_enrolled::StudentEnrolled,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::StudentEnrolled::project for CourseRoster")
    }
}
impl Project<
    crate::generated::domain::student_course_enrollment::event::enrollment_dropped::EnrollmentDropped,
> for CourseRoster {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::student_course_enrollment::event::enrollment_dropped::EnrollmentDropped,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::EnrollmentDropped::project for CourseRoster")
    }
}
