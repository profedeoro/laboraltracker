use crate::domain::error::AppError;
use crate::domain::ports::{Clock, TaskRepository, TimeSessionRepository};
use crate::domain::time_session::TimeSession;

/// Caso de uso: iniciar el cronómetro de una tarea. Dueño del invariante de sesión
/// única: si hay una sesión corriendo, la cierra antes de abrir la nueva.
pub struct StartTimerUseCase;

impl StartTimerUseCase {
    pub fn execute(
        sessions: &mut impl TimeSessionRepository,
        tasks: &impl TaskRepository,
        clock: &impl Clock,
        task_id: String,
    ) -> Result<TimeSession, AppError> {
        if tasks.find_by_id(&task_id)?.is_none() {
            return Err(AppError::NotFound(task_id));
        }
        let now = clock.now();
        // Cerrar la sesión activa (si la hay) antes de abrir otra.
        if let Some(mut running) = sessions.running()? {
            running.stop(now)?;
            sessions.update(&running)?;
        }
        let session = TimeSession::start(ulid::Ulid::new().to_string(), task_id, now);
        sessions.add(&session)?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{
        FixedClock, InMemoryTaskRepository, InMemoryTimeSessionRepository,
    };
    use crate::domain::task::Task;

    fn tasks_with_t1() -> InMemoryTaskRepository {
        let mut r = InMemoryTaskRepository::default();
        r.items
            .push(Task::new("t1".into(), "p1".into(), "Tarea 1".into(), 1).unwrap());
        r.items
            .push(Task::new("t2".into(), "p1".into(), "Tarea 2".into(), 2).unwrap());
        r
    }

    #[test]
    fn starts_a_session_when_none_running() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let tasks = tasks_with_t1();
        let clock = FixedClock(1000);
        let s = StartTimerUseCase::execute(&mut sessions, &tasks, &clock, "t1".into()).unwrap();
        assert_eq!(s.task_id, "t1");
        assert_eq!(s.started_at, 1000);
        assert_eq!(s.ended_at, None);
        assert_eq!(sessions.items.len(), 1);
    }

    #[test]
    fn starting_another_closes_the_running_one() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let tasks = tasks_with_t1();

        let first =
            StartTimerUseCase::execute(&mut sessions, &tasks, &FixedClock(1000), "t1".into())
                .unwrap();
        let second =
            StartTimerUseCase::execute(&mut sessions, &tasks, &FixedClock(2000), "t2".into())
                .unwrap();

        assert_eq!(sessions.items.len(), 2);
        let running: Vec<_> = sessions
            .items
            .iter()
            .filter(|s| s.ended_at.is_none())
            .collect();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, second.id);
        let closed = sessions.items.iter().find(|s| s.id == first.id).unwrap();
        assert_eq!(closed.ended_at, Some(2000));
    }

    #[test]
    fn rejects_start_on_missing_task() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let tasks = tasks_with_t1();
        let r =
            StartTimerUseCase::execute(&mut sessions, &tasks, &FixedClock(0), "ghost".into());
        assert!(matches!(r, Err(AppError::NotFound(id)) if id == "ghost"));
        assert_eq!(sessions.items.len(), 0);
    }
}
