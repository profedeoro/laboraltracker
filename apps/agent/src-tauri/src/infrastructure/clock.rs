use crate::domain::ports::Clock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Reloj real de pared en epoch millis UTC.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        // Fallback a 0 con log si el reloj está antes de UNIX epoch.
        // La invariante es "no panic en flujo normal".
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|e| {
                eprintln!("[laboraltracker] clock before unix epoch: {e}");
                std::time::Duration::ZERO
            })
            .as_millis() as i64
    }
}
