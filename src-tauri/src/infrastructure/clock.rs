use crate::domain::ports::Clock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Reloj real de pared en epoch millis UTC.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_millis() as i64
    }
}
