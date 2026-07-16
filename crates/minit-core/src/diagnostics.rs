use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub sequence: u64,
    pub scope: String,
    pub message: String,
}

impl DiagnosticEvent {
    pub fn new(scope: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            sequence: 0,
            scope: scope.into(),
            message: message.into(),
        }
    }

    pub fn sequenced(sequence: u64, scope: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            sequence,
            scope: scope.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DiagnosticEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "[{}] #{} {}",
            self.scope, self.sequence, self.message
        )
    }
}

#[derive(Debug, Clone)]
pub struct EventBuffer {
    capacity: usize,
    next_sequence: u64,
    events: VecDeque<DiagnosticEvent>,
}

impl EventBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            next_sequence: 1,
            events: VecDeque::new(),
        }
    }

    pub fn record(
        &mut self,
        scope: impl Into<String>,
        message: impl Into<String>,
    ) -> DiagnosticEvent {
        let event = DiagnosticEvent::sequenced(self.next_sequence, scope, message);
        self.next_sequence += 1;
        if self.capacity > 0 && self.events.len() == self.capacity {
            self.events.pop_front();
        }
        if self.capacity > 0 {
            self.events.push_back(event.clone());
        }
        event
    }

    pub fn recent(&self) -> Vec<DiagnosticEvent> {
        self.events.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_event_formats_scope_and_message() {
        let event = DiagnosticEvent::new("boot", "mounted /proc");

        assert_eq!(event.sequence, 0);
        assert_eq!(event.scope, "boot");
        assert_eq!(event.message, "mounted /proc");
        assert_eq!(event.to_string(), "[boot] #0 mounted /proc");
    }

    #[test]
    fn event_buffer_assigns_sequence_and_keeps_recent_events() {
        let mut buffer = EventBuffer::new(2);

        buffer.record("boot", "mounted /proc");
        buffer.record("control", "started sshd");
        buffer.record("shutdown", "stopped sshd");

        assert_eq!(
            buffer.recent(),
            vec![
                DiagnosticEvent::sequenced(2, "control", "started sshd"),
                DiagnosticEvent::sequenced(3, "shutdown", "stopped sshd"),
            ]
        );
    }
}
