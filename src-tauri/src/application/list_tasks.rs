use crate::domain::error::AppError;
use crate::domain::ports::TaskRepository;
use crate::domain::task::Task;

/// Caso de uso: listar las tareas de un proyecto.
pub struct ListTasksUseCase;

impl ListTasksUseCase {
    pub fn execute(
        repo: &impl TaskRepository,
        project_id: &str,
    ) -> Result<Vec<Task>, AppError> {
        repo.list_by_project(project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::InMemoryTaskRepository;

    #[test]
    fn lists_only_tasks_of_the_given_project() {
        let mut repo = InMemoryTaskRepository::default();
        repo.items.push(Task::new("t1".into(), "p1".into(), "A".into(), 1).unwrap());
        repo.items.push(Task::new("t2".into(), "p2".into(), "B".into(), 2).unwrap());
        repo.items.push(Task::new("t3".into(), "p1".into(), "C".into(), 3).unwrap());
        let out = ListTasksUseCase::execute(&repo, "p1").unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "A");
        assert_eq!(out[1].name, "C");
    }
}
