use crate::domain::error::AppError;
use crate::domain::ports::ProjectRepository;
use crate::domain::project::Project;

/// Caso de uso: listar todos los proyectos.
pub struct ListProjectsUseCase;

impl ListProjectsUseCase {
    pub fn execute(repo: &impl ProjectRepository) -> Result<Vec<Project>, AppError> {
        repo.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::InMemoryProjectRepository;

    #[test]
    fn lists_all_projects() {
        let mut repo = InMemoryProjectRepository::default();
        repo.items.push(Project::new("p1".into(), "A".into(), None, 1).unwrap());
        repo.items.push(Project::new("p2".into(), "B".into(), None, 2).unwrap());
        let out = ListProjectsUseCase::execute(&repo).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "A");
    }
}
