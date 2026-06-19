use crate::domain::error::AppError;
use crate::domain::ports::{Clock, TimeSessionRepository};
use crate::domain::time_session::TimeSession;

/// Caso de uso: parar la sesión activa. Error si no hay ninguna.
pub struct StopTimerUseCase;

impl StopTimerUseCase {
    pub fn execute(
        sessions: &mut impl TimeSessionRepository,
        clock: &impl Clock,
    ) -> Result<TimeSession, AppError> {
        let mut running = sessions.running()?.ok_or(AppError::NoRunningSession)?;
        running.stop(clock.now())?;
        sessions.update(&running)?;
        Ok(running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::testing::{FixedClock, InMemoryTimeSessionRepository};
    use crate::domain::time_session::TimeSession;

    #[test]
    fn stops_the_running_session() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        sessions.items.push(TimeSession::start("s1".into(), "t1".into(), 1000));
        let stopped = StopTimerUseCase::execute(&mut sessions, &FixedClock(4000)).unwrap();
        assert_eq!(stopped.ended_at, Some(4000));
        assert!(sessions.running().unwrap().is_none(), "no debe quedar sesión activa");
    }

    #[test]
    fn errors_when_no_running_session() {
        let mut sessions = InMemoryTimeSessionRepository::default();
        let r = StopTimerUseCase::execute(&mut sessions, &FixedClock(0));
        assert!(matches!(r, Err(AppError::NoRunningSession)));
    }
}
