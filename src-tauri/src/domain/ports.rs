use crate::domain::error::AppError;
use crate::domain::project::Project;

/// Puerto de persistencia de proyectos. La infraestructura lo implementa.
pub trait ProjectRepository {
    fn add(&mut self, project: &Project) -> Result<(), AppError>;
    fn list(&self) -> Result<Vec<Project>, AppError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Project>, AppError>;
}

/// Reloj inyectable. now() = epoch millis UTC (ver 02-time-policy.md).
pub trait Clock {
    fn now(&self) -> i64;
}
