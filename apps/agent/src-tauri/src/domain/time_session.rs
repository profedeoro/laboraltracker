use crate::domain::error::AppError;

/// Cap de duración (configurable a futuro). Una sesión más larga se marca sospechosa.
const MAX_SESSION_MS: i64 = 12 * 60 * 60 * 1000; // 12 h

/// Entidad de dominio: una sesión de cronómetro sobre una tarea.
/// Instantes en epoch-millis UTC. Duración derivada, nunca persistida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSession {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
    pub is_suspect: bool,
}

impl TimeSession {
    /// Abre una sesión nueva (en curso). Id = ULID (texto), generado fuera.
    pub fn start(id: String, task_id: String, now: i64) -> Self {
        Self {
            id,
            task_id,
            started_at: now,
            ended_at: None,
            last_heartbeat_at: None,
            is_suspect: false,
        }
    }

    /// Cierra la sesión en `now`. Rechaza reloj retrocedido (`now < started_at`).
    /// Si la duración supera el cap, marca la sesión como sospechosa.
    pub fn stop(&mut self, now: i64) -> Result<(), AppError> {
        if now < self.started_at {
            return Err(AppError::ClockWentBackwards);
        }
        self.ended_at = Some(now);
        if now - self.started_at > MAX_SESSION_MS {
            self.is_suspect = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_open_and_not_suspect() {
        let s = TimeSession::start("s1".into(), "t1".into(), 1000);
        assert_eq!(s.started_at, 1000);
        assert_eq!(s.ended_at, None);
        assert!(!s.is_suspect);
        assert_eq!(s.last_heartbeat_at, None);
    }

    #[test]
    fn stop_sets_ended_at() {
        let mut s = TimeSession::start("s1".into(), "t1".into(), 1000);
        s.stop(5000).unwrap();
        assert_eq!(s.ended_at, Some(5000));
        assert!(!s.is_suspect);
    }

    #[test]
    fn stop_rejects_clock_backwards_and_leaves_session_open() {
        let mut s = TimeSession::start("s1".into(), "t1".into(), 1000);
        let r = s.stop(999);
        assert!(matches!(r, Err(AppError::ClockWentBackwards)));
        assert_eq!(s.ended_at, None, "no debe cerrar si el reloj retrocedió");
    }

    #[test]
    fn stop_marks_suspect_when_over_cap() {
        let mut s = TimeSession::start("s1".into(), "t1".into(), 0);
        let over = MAX_SESSION_MS + 1;
        s.stop(over).unwrap();
        assert_eq!(s.ended_at, Some(over));
        assert!(s.is_suspect, "una sesión > 12 h debe marcarse sospechosa");
    }
}
