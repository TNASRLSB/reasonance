import { describe, it } from 'vitest';

describe('agents store', () => {
  it.todo('discoveredAgents starts as empty array');
  it.todo('activeAgents starts as empty array');
  it.todo('agentMessages starts as empty array');
  it.todo('runningAgentCount derived store returns 0 when no agents are active');
  it.todo('runningAgentCount counts only agents with state "running"');
  it.todo('erroredAgentCount returns 0 when no agents have errored');
  it.todo('erroredAgentCount counts agents with state "error" or "failed"');
  it.todo('runningAgentCount and erroredAgentCount update reactively when activeAgents changes');
});
