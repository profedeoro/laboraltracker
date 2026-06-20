use crate::domain::error::AppError;
use crate::domain::ports::{Clock, TimeSessionRepository};
use crate::domain::time_session::TimeSession;

/// Caso de uso: actualizar `last_heartbeat_at` de la sesión activa.
/// Si no hay sesión corriendo, retorna `Ok(None)` sin error.
pub struct HeartbeatUseCase;

impl HeartbeatUseCase {
    pub fn execute(
        sessions: &mut impl TimeSessionRepository,
        clock: &impl Clock,
    ) -> Result<Option<TimeSession>, AppError> {
        let mut running = match sessions.running()? {
            Some(s) => s,
            None => return Ok(None),
        };
        running.last_heartbeat_at = Some(clock.now());
        sessions.update(&running)?;
        Ok(Some(running))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{FixedClock, InMemoryTimeSessionRepository};
    use crate::domain::time_session::TimeSession;

    #[test]
    fn updates_heartbeat_on_running_session() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        sessions.items.push(TimeSession::start("s1".into(), "t1".into(), 1000));
        let clock = FixedClock(5000);

        let result = HeartbeatUseCase::execute(&mut sessions, &clock).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().last_heartbeat_at, Some(5000));
    }

    #[test]
    fn returns_none_when_no_running_session() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let result = HeartbeatUseCase::execute(&mut sessions, &FixedClock(0)).unwrap();
        assert!(result.is_none());
    }
}
