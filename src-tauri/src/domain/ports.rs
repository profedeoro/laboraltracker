use crate::domain::error::AppError;
use crate::domain::project::Project;

/// Puerto de persistencia de proyectos. La infraestructura lo implementa.
pub trait ProjectRepository {
    fn add(&mut self, project: &Project) -> Result<(), AppError>;
    fn list(&self) -> Result<Vec<Project>, AppError>;
    // Forward-declared for Plan 3 (tasks); part of the contract.
    #[allow(dead_code)]
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
}
