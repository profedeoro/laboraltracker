use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository, TaskRepository, TimeSessionRepository};
use crate::domain::project::Project;
use crate::domain::task::Task;
use crate::domain::time_session::TimeSession;

/// Repositorio en memoria para tests de casos de uso (sin `SQLite`).
#[derive(Default)]
pub struct InMemoryProjectRepository {
    pub items: Vec<Project>,
}

impl ProjectRepository for InMemoryProjectRepository {
    fn add(&mut self, project: &Project) -> Result<(), AppError> {
        self.items.push(project.clone());
        Ok(())
    }
    fn list(&self) -> Result<Vec<Project>, AppError> {
        Ok(self.items.clone())
    }
    fn find_by_id(&self, id: &str) -> Result<Option<Project>, AppError> {
        Ok(self.items.iter().find(|p| p.id == id).cloned())
    }
}

/// Reloj fijo para tests deterministas.
pub struct FixedClock(pub i64);
impl Clock for FixedClock {
    fn now(&self) -> i64 {
        self.0
    }
}

/// Repositorio de tareas en memoria para tests de casos de uso (sin SQLite).
#[derive(Default)]
pub struct InMemoryTaskRepository {
    pub items: Vec<Task>,
}

impl TaskRepository for InMemoryTaskRepository {
    fn add(&mut self, task: &Task) -> Result<(), AppError> {
        self.items.push(task.clone());
        Ok(())
    }
    fn list_by_project(&self, project_id: &str) -> Result<Vec<Task>, AppError> {
        Ok(self
            .items
            .iter()
            .filter(|t| t.project_id == project_id)
            .cloned()
            .collect())
    }
    fn find_by_id(&self, id: &str) -> Result<Option<Task>, AppError> {
        Ok(self.items.iter().find(|t| t.id == id).cloned())
    }
}

/// Repositorio de sesiones en memoria para tests de casos de uso (sin SQLite).
#[derive(Default)]
pub struct InMemoryTimeSessionRepository {
    pub items: Vec<TimeSession>,
}

impl TimeSessionRepository for InMemoryTimeSessionRepository {
    fn running(&self) -> Result<Option<TimeSession>, AppError> {
        Ok(self.items.iter().find(|s| s.ended_at.is_none()).cloned())
    }
    fn add(&mut self, session: &TimeSession) -> Result<(), AppError> {
        self.items.push(session.clone());
        Ok(())
    }
    fn update(&mut self, session: &TimeSession) -> Result<(), AppError> {
        match self.items.iter_mut().find(|s| s.id == session.id) {
            Some(slot) => {
                *slot = session.clone();
                Ok(())
            }
            None => Err(AppError::NotFound(session.id.clone())),
        }
    }
}
