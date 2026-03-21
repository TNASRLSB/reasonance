import { describe, it } from 'vitest';

describe('engine store', () => {
  it.todo('currentRun starts as null');
  it.todo('currentRunId starts as null');
  it.todo('runStatus is "idle" when currentRun is null');
  it.todo('runStatus reflects the status field of the current run');
  it.todo('nodeStates is empty array when currentRun is null');
  it.todo('nodeStates returns all node_states values from current run');
  it.todo('completedNodeCount counts nodes with state "success"');
  it.todo('totalNodeCount returns total number of node states');
  it.todo('activeNodeCount counts nodes with state "running"');
  it.todo('errorNodeCount counts nodes with state "error"');
  it.todo('statusSummary formats completed/total/active/error counts as a string');
  it.todo('statusSummary omits active and error parts when counts are zero');
});
