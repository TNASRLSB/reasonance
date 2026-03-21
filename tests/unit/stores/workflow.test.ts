import { describe, it } from 'vitest';

describe('workflow store', () => {
  it.todo('currentWorkflow starts as null');
  it.todo('currentWorkflowPath starts as null');
  it.todo('workflowDirty starts as false');
  it.todo('workflowList starts as empty array');
  it.todo('nodeCount derived store returns 0 when currentWorkflow is null');
  it.todo('nodeCount derived store returns correct count from workflow nodes');
  it.todo('edgeCount derived store returns 0 when currentWorkflow is null');
  it.todo('edgeCount derived store returns correct count from workflow edges');
  it.todo('setting currentWorkflow updates nodeCount and edgeCount reactively');
});
