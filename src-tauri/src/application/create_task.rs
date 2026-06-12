use crate::domain::error::AppError;
use crate::domain::ports::{Clock, ProjectRepository, TaskRepository};
use crate::domain::task::Task;

/// Caso de uso: crear una tarea dentro de un proyecto EXISTENTE.
/// Verifica la existencia del proyecto (`NotFound` si falta), genera el id ULID y
/// sella `created_at` con el `Clock`. Defensa en profundidad: la FK de la BD también
/// lo rechaza, pero aquí damos un error de dominio claro.
pub struct CreateTaskUseCase;

impl CreateTaskUseCase {
    pub fn execute(
        tasks: &mut impl TaskRepository,
        projects: &impl ProjectRepository,
        clock: &impl Clock,
        project_id: String,
        name: String,
    ) -> Result<Task, AppError> {
        if projects.find_by_id(&project_id)?.is_none() {
            return Err(AppError::NotFound(project_id));
        }
        let id = ulid::Ulid::new().to_string();
        let task = Task::new(id, project_id, name, clock.now())?;
        tasks.add(&task)?;
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{
        FixedClock, InMemoryProjectRepository, InMemoryTaskRepository,
    };
    use crate::domain::project::Project;

    fn projects_with_p1() -> InMemoryProjectRepository {
        let mut r = InMemoryProjectRepository::default();
        r.items
            .push(Project::new("p1".into(), "Cliente A".into(), None, 1).unwrap());
        r
    }

    #[test]
    fn creates_task_under_existing_project() {
        let mut tasks = InMemoryTaskRepository::default();
        let projects = projects_with_p1();
        let clock = FixedClock(1000);
        let t = CreateTaskUseCase::execute(
            &mut tasks,
            &projects,
            &clock,
            "p1".into(),
            "Disenar".into(),
        )
        .unwrap();
        assert_eq!(t.project_id, "p1");
        assert_eq!(t.created_at, 1000);
        assert!(!t.id.is_empty());
        assert_eq!(tasks.items.len(), 1);
    }

    #[test]
    fn rejects_when_project_missing() {
        let mut tasks = InMemoryTaskRepository::default();
        let projects = InMemoryProjectRepository::default(); // vacío
        let clock = FixedClock(0);
        let r = CreateTaskUseCase::execute(
            &mut tasks,
            &projects,
            &clock,
            "ghost".into(),
            "X".into(),
        );
        assert!(matches!(r, Err(AppError::NotFound(id)) if id == "ghost"));
        assert_eq!(tasks.items.len(), 0);
    }

    #[test]
    fn rejects_empty_name_and_persists_nothing() {
        let mut tasks = InMemoryTaskRepository::default();
        let projects = projects_with_p1();
        let clock = FixedClock(0);
        let r = CreateTaskUseCase::execute(
            &mut tasks,
            &projects,
            &clock,
            "p1".into(),
            "  ".into(),
        );
        assert!(matches!(r, Err(AppError::TaskNameEmpty)));
        assert_eq!(tasks.items.len(), 0);
    }
}
