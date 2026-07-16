use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub scope: String,
    pub message: String,
}

impl DiagnosticEvent {
    pub fn new(scope: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DiagnosticEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[{}] {}", self.scope, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_event_formats_scope_and_message() {
        let event = DiagnosticEvent::new("boot", "mounted /proc");

        assert_eq!(event.scope, "boot");
        assert_eq!(event.message, "mounted /proc");
        assert_eq!(event.to_string(), "[boot] mounted /proc");
    }
}
