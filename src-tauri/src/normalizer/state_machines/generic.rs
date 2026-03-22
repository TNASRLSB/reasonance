use super::StateMachine;
use crate::agent_event::AgentEvent;

/// Pass-through state machine. No accumulation — every event emits immediately.
/// Used for providers that don't have multi-part event sequences.
pub struct GenericStateMachine;

impl GenericStateMachine {
    pub fn new() -> Self {
        Self
    }
}

impl StateMachine for GenericStateMachine {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent> {
        vec![event]
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_event::AgentEvent;

    #[test]
    fn test_generic_passes_through() {
        let mut sm = GenericStateMachine::new();
        let event = AgentEvent::text("hello", "test");
        let result = sm.process(event.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, event.id);
    }

    #[test]
    fn test_generic_reset_is_noop() {
        let mut sm = GenericStateMachine::new();
        sm.reset(); // should not panic
    }
}
