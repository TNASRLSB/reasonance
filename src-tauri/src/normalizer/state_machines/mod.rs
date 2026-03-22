pub mod generic;
pub mod claude;

use crate::agent_event::AgentEvent;

/// Trait for provider-specific event accumulation.
/// Some providers emit events in sequences that need assembly
/// (e.g., Claude's content_block_start → N deltas → stop).
///
/// Returns 0, 1, or N events:
/// - 0: accumulating, not ready to emit yet
/// - 1: single event ready
/// - N: flushing accumulated events
pub trait StateMachine: Send + Sync {
    fn process(&mut self, event: AgentEvent) -> Vec<AgentEvent>;
    fn reset(&mut self);
}
