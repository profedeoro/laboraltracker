use crate::domain::error::AppError;
use crate::domain::ports::{Clock, TimeSessionRepository};

/// Caso de uso: recuperar sesiones huérfanas al arrancar.
///
/// Toda sesión con `ended_at IS NULL` se cierra en `last_heartbeat_at` (si existe)
/// o en `started_at`, y se marca `is_suspect = 1` para no contaminar reportes.
/// Se invoca desde `lib.rs::setup` ANTES de `app.manage(...)`.
pub struct RecoverOrphanSessionsUseCase;

impl RecoverOrphanSessionsUseCase {
    pub fn execute(
        sessions: &mut impl TimeSessionRepository,
        clock: &impl Clock,
    ) -> Result<usize, AppError> {
        let running = sessions.list_running()?;
        let count = running.len();
        for mut session in running {
            let close_at = session.last_heartbeat_at.unwrap_or(session.started_at);
            session.ended_at = Some(close_at);
            session.last_heartbeat_at = Some(clock.now());
            session.is_suspect = true;
            sessions.update(&session)?;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{FixedClock, InMemoryTimeSessionRepository};
    use crate::domain::time_session::TimeSession;

    #[test]
    fn recovers_single_orphan() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        sessions.items.push(TimeSession::start("s1".into(), "t1".into(), 1000));
        let count =
            RecoverOrphanSessionsUseCase::execute(&mut sessions, &FixedClock(9999)).unwrap();
        assert_eq!(count, 1);
        assert!(sessions.running().unwrap().is_none(), "no debe quedar activa");
        let recovered = &sessions.items[0];
        assert!(recovered.is_suspect);
        assert_eq!(recovered.ended_at, Some(1000)); // sin heartbeat → started_at
    }

    #[test]
    fn recovers_orphan_with_heartbeat() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let mut s = TimeSession::start("s1".into(), "t1".into(), 1000);
        s.last_heartbeat_at = Some(8000);
        sessions.items.push(s);
        let count =
            RecoverOrphanSessionsUseCase::execute(&mut sessions, &FixedClock(9999)).unwrap();
        assert_eq!(count, 1);
        let recovered = &sessions.items[0];
        assert!(recovered.is_suspect);
        assert_eq!(recovered.ended_at, Some(8000)); // heartbeat → last_heartbeat_at
    }

    #[test]
    fn noop_when_none_running() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let count =
            RecoverOrphanSessionsUseCase::execute(&mut sessions, &FixedClock(0)).unwrap();
        assert_eq!(count, 0);
    }
}
