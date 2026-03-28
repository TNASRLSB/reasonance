import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as tauriCore from '@tauri-apps/api/core';
import { TauriAdapter } from '$lib/adapter/tauri';

describe('TauriAdapter batching', () => {
  let adapter: TauriAdapter;
  let invokeSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.restoreAllMocks();
    invokeSpy = vi.spyOn(tauriCore, 'invoke');
    adapter = new TauriAdapter();
  });

  it('batches calls in same microtask into a single batch_invoke', async () => {
    invokeSpy.mockResolvedValueOnce([
      { ok: 'hello', err: null },
      { ok: 'world', err: null },
    ]);

    const p1 = adapter.readFile('/a.txt');
    const p2 = adapter.readFile('/b.txt');

    const [r1, r2] = await Promise.all([p1, p2]);

    expect(r1).toBe('hello');
    expect(r2).toBe('world');
    expect(invokeSpy).toHaveBeenCalledTimes(1);
    expect(invokeSpy).toHaveBeenCalledWith('batch_invoke', {
      calls: [
        { command: 'read_file', args: { path: '/a.txt' } },
        { command: 'read_file', args: { path: '/b.txt' } },
      ],
    });
  });

  it('deduplicates identical calls', async () => {
    invokeSpy.mockResolvedValueOnce([
      { ok: 'content', err: null },
    ]);

    const p1 = adapter.readFile('/same.txt');
    const p2 = adapter.readFile('/same.txt');

    const [r1, r2] = await Promise.all([p1, p2]);

    expect(r1).toBe('content');
    expect(r2).toBe('content');
    expect(invokeSpy).toHaveBeenCalledTimes(1);
    // Only 1 call in the batch due to dedup
    const batchArgs = invokeSpy.mock.calls[0][1] as { calls: unknown[] };
    expect(batchArgs.calls).toHaveLength(1);
  });

  it('rejects on Rust error', async () => {
    invokeSpy.mockResolvedValueOnce([
      { ok: null, err: 'file not found' },
    ]);

    await expect(adapter.readFile('/missing.txt')).rejects.toBe('file not found');
  });

  it('handles partial failures', async () => {
    invokeSpy.mockResolvedValueOnce([
      { ok: 'success-content', err: null },
      { ok: null, err: 'permission denied' },
    ]);

    const p1 = adapter.readFile('/good.txt');
    const p2 = adapter.readFile('/bad.txt');

    await expect(p1).resolves.toBe('success-content');
    await expect(p2).rejects.toBe('permission denied');
  });

  it('long-running commands bypass batching (agentSend calls invoke directly)', async () => {
    invokeSpy.mockResolvedValueOnce('session-123');

    const result = await adapter.agentSend('hello', 'claude');

    expect(result).toBe('session-123');
    expect(invokeSpy).toHaveBeenCalledWith('agent_send', expect.objectContaining({
      request: expect.objectContaining({ prompt: 'hello', provider: 'claude' }),
    }));
    // It should call invoke directly, not batch_invoke
    expect(invokeSpy.mock.calls[0][0]).toBe('agent_send');
  });

  it('rejects immediately when signal is already aborted', async () => {
    const controller = new AbortController();
    controller.abort();

    await expect(adapter.readFile('/a.txt', controller.signal)).rejects.toThrow();
    // Should not even reach batch_invoke
    expect(invokeSpy).not.toHaveBeenCalled();
  });

  it('rejects with ZodError when result fails schema validation', async () => {
    // read_file schema expects z.string(), send a number instead
    invokeSpy.mockResolvedValueOnce([
      { ok: 12345, err: null },
    ]);

    await expect(adapter.readFile('/test.txt')).rejects.toBeDefined();
  });

  it('discards result for calls aborted during flight', async () => {
    const controller = new AbortController();

    // Mock invoke to abort the signal mid-flight, then return a result
    invokeSpy.mockImplementation(async () => {
      controller.abort();
      return [{ ok: 'stale-data', err: null }];
    });

    // readFile with signal — the abort fires during invoke, before result distribution
    await expect(adapter.readFile('/test.txt', controller.signal)).rejects.toThrow();
  });

  it('explicit batch() sends all captured calls together', async () => {
    invokeSpy.mockResolvedValueOnce([
      { ok: 'file-content', err: null },
      { ok: null, err: null },
    ]);

    const [content] = await adapter.batch((ctx) => [
      ctx.readFile('/test.txt'),
      ctx.writeFile('/out.txt', 'data'),
    ]);

    expect(content).toBe('file-content');
    expect(invokeSpy).toHaveBeenCalledTimes(1);
    expect(invokeSpy).toHaveBeenCalledWith('batch_invoke', {
      calls: [
        { command: 'read_file', args: { path: '/test.txt' } },
        { command: 'write_file', args: { path: '/out.txt', content: 'data' } },
      ],
    });
  });
});
