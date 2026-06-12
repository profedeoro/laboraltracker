use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository};
use crate::domain::project::Project;

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
