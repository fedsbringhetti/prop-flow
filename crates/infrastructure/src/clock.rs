use chrono::{DateTime, Utc};
use propflow_domain::ports::Clock;

/// Orologio di sistema. In test si sostituisce con un orologio fisso.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
