/// A latching timeout object we can later use to determine if a timeout was reached.
use std::time::{Duration, Instant};

pub(crate) enum Timeout {
    None,
    Future(Instant),
    TimedOut,
}


impl Timeout {
    pub(crate) fn set(&mut self, limit: Option<Duration>) {
        *self = match limit {
            Some(dur) => Timeout::Future(Instant::now() + dur),
            None => Timeout::None,
        }
    }

    // Checks if timer expired and set timed-out latch if so
    pub(crate) fn is_timed_out(&mut self) -> bool {
        if let Timeout::Future(t) = self {
            if Instant::now() > *t {
                *self = Timeout::TimedOut;
            }
        }
        self.timed_out()
    }

    /// Check if a previous operation detected a timeout
    pub(crate) fn timed_out(&self) -> bool {
        matches!(self, Timeout::TimedOut)
    }
}
