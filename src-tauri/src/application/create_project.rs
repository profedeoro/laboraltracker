use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository};
use crate::domain::project::Project;

/// Caso de uso: crear un proyecto. Genera el id ULID y sella created_at con el Clock.
pub struct CreateProjectUseCase;

impl CreateProjectUseCase {
    pub fn execute(
        repo: &mut impl ProjectRepository,
        clock: &impl Clock,
        name: String,
        color: Option<String>,
    ) -> Result<Project, AppError> {
        let id = ulid::Ulid::new().to_string();
        let project = Project::new(id, name, color, clock.now())?;
        repo.add(&project)?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{FixedClock, InMemoryProjectRepository};

    #[test]
    fn creates_and_persists_project() {
        let mut repo = InMemoryProjectRepository::default();
        let clock = FixedClock(1000);
        let p = CreateProjectUseCase::execute(&mut repo, &clock, "Cliente A".into(), None).unwrap();
        assert_eq!(p.name, "Cliente A");
        assert_eq!(p.created_at, 1000);
        assert!(!p.id.is_empty());
        assert_eq!(repo.list().unwrap().len(), 1);
    }

    #[test]
    fn rejects_empty_name_and_persists_nothing() {
        let mut repo = InMemoryProjectRepository::default();
        let clock = FixedClock(0);
        let r = CreateProjectUseCase::execute(&mut repo, &clock, "  ".into(), None);
        assert!(matches!(r, Err(AppError::ProjectNameEmpty)));
        assert_eq!(repo.list().unwrap().len(), 0);
    }
}
