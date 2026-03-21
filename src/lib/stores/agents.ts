import { writable, derived } from 'svelte/store';
import type { AgentInstance, AgentMessage, DiscoveredAgent } from '$lib/adapter/index';

export const discoveredAgents = writable<DiscoveredAgent[]>([]);
export const activeAgents = writable<AgentInstance[]>([]);
export const agentMessages = writable<AgentMessage[]>([]);
export const runningAgentCount = derived(activeAgents, ($agents) =>
  $agents.filter(a => a.state === 'running').length
);
export const erroredAgentCount = derived(activeAgents, ($agents) =>
  $agents.filter(a => a.state === 'error' || a.state === 'failed').length
);
