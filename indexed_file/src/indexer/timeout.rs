/// A latching timeout object we can later use to determine if a timeout was reached.
use std::time::{Duration, Instant};

pub(crate) enum Timeout {
    Future(Instant),
    TimedOut,
    Inactive(bool),     // bool holds status of previous timeout
}


impl Timeout {
    pub(crate) fn set(&mut self, limit: Option<Duration>) {
        *self = match limit {
            Some(dur) => Timeout::Future(Instant::now() + dur),
            None => Timeout::Inactive(self.timed_out()),
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

    /// Check if the current operation is timed out
    pub(crate) fn timed_out(&self) -> bool {
        matches!(self, Timeout::TimedOut)
    }

    // Check if the previous operation timed out.  Resets on the next operation.
    pub(crate) fn prev_timed_out(&self) -> bool {
        if let Timeout::Inactive(b) = self {
            *b
        } else {
            false
        }
    }

    // Reset the previous timeout history when we start a new action
    pub(crate) fn active(&mut self) {
        if let Timeout::Inactive(_) = self {
            *self = Timeout::Inactive(false);
        }
    }
}


use crate::IndexedLog;

use super::{waypoint::Position, GetLine};

pub struct TimeoutWrapper<'a, LOG: IndexedLog>  {
    inner: &'a mut LOG,
}

impl<'a, LOG: IndexedLog> TimeoutWrapper<'a, LOG> {
    pub fn new(inner: &'a mut LOG, ms: usize) -> Self {
        inner.set_timeout(Some(Duration::from_millis(ms as u64)));
        Self { inner }
    }
}

impl<'a, LOG: IndexedLog> Drop for TimeoutWrapper<'a, LOG> {
    fn drop(&mut self) {
        self.inner.set_timeout(None);
    }
}

impl<'a, LOG: IndexedLog> IndexedLog for TimeoutWrapper<'a, LOG> {
    fn next(&mut self, pos: &Position) -> GetLine {
        self.inner.next(pos)
    }

    fn read_line(&mut self, offset: usize) -> Option<crate::LogLine> {
        self.inner.read_line(offset)
    }

    fn next_back(&mut self, pos: &super::waypoint::Position) -> super::GetLine {
        self.inner.next_back(pos)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn indexed_bytes(&self) -> usize {
        self.inner.indexed_bytes()
    }

    fn count_lines(&self) -> usize  {
        self.inner.count_lines()
    }

    fn set_timeout(&mut self, limit: Option<Duration>) {
        self.inner.set_timeout(limit);
    }

    fn timed_out(&mut self) -> bool {
        self.inner.timed_out()
    }
}
