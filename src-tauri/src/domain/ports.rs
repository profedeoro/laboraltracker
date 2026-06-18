use crate::domain::error::AppError;
use crate::domain::project::Project;

/// Puerto de persistencia de proyectos. La infraestructura lo implementa.
pub trait ProjectRepository {
    fn add(&mut self, project: &Project) -> Result<(), AppError>;
    fn list(&self) -> Result<Vec<Project>, AppError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Project>, AppError>;
}

/// Reloj inyectable. `now()` = epoch millis UTC (ver 02-time-policy.md).
pub trait Clock {
    fn now(&self) -> i64;
}

use crate::domain::task::Task;

/// Puerto de persistencia de tareas. La infraestructura lo implementa.
pub trait TaskRepository {
    fn add(&mut self, task: &Task) -> Result<(), AppError>;
    fn list_by_project(&self, project_id: &str) -> Result<Vec<Task>, AppError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Task>, AppError>;
}

use crate::domain::time_session::TimeSession;

/// Puerto de persistencia de sesiones de cronómetro.
pub trait TimeSessionRepository {
    /// La sesión actualmente en curso (`ended_at IS NULL`), si existe.
    fn running(&self) -> Result<Option<TimeSession>, AppError>;
    fn add(&mut self, session: &TimeSession) -> Result<(), AppError>;
    fn update(&mut self, session: &TimeSession) -> Result<(), AppError>;
}
