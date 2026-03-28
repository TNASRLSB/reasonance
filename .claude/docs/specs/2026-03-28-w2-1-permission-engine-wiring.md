# W2.1 — Permission Engine Wiring Design

**Date**: 2026-03-28
**Status**: Approved
**Depends on**: W1.1 (EventBus), W1.4 (LayeredSettings), W1.5 (Structured errors)
**Blocks**: W2.2 (Permission timeout UI), W2.3 (Per-tool approval persistence)

## Problem

The permission engine (`permission_engine.rs`, 657 LOC, 17 tests) exists but is completely bypassed. Transport (`transport/mod.rs`) has ~35 lines of inline trust checks that don't call `evaluate()`. The frontend handles permissions with all-or-nothing approval that replays the entire message. Layer 3 (policy file) and Layer 5 (session memory) are unimplemented or unintegrated.

## Solution

Wire `PermissionEngine::evaluate()` as the sole decision point in transport. Implement Layer 3 (policy file parsing) and Layer 5 (session memory lookup). Redesign the frontend approval UI for per-tool granularity with 4 scope levels. Eliminate all inline permission checks from transport.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Engine placement | Pre-CLI (builds --allowedTools) + post-CLI (handles denials) | Pre-filter reduces unnecessary denials; post-filter gives rich approval UX |
| Layer 1 in yolo mode | Always active | `rm -rf /` and force-push to main are blocked unconditionally |
| Session memory in evaluate | Layer 5 inside evaluate(), not external | Single pass through all layers, consistent priority ordering |
| Project scope persistence | Writes to permissions.toml (Layer 3 file) | Decisions survive app restart without duplicating storage |
| Frontend approval | Per-tool with 4 scopes | Matches Claude Code's granularity: once, session, project, deny |
| Resume after approval | --resume if CLI supports, full replay otherwise | Best UX when possible, graceful fallback |

## Architecture

### Backend: Engine as sole decision point

#### Updated `evaluate()` signature

```rust
pub fn evaluate(
    &self,
    ctx: &PermissionContext,
    memory: &PermissionMemory,
) -> PermissionDecision
```

#### Layer evaluation order

1. **Layer 1 — Hardcoded rules** (non-overridable, even in yolo):
   - `rm -rf /`, `rm -rf ~`, `rm -rf .`, `chmod 777` → Deny always
   - `git push --force` to main/master → Deny always
   - Write/Edit outside project_root → Deny always

2. **Layer 2 — Trust level**:
   - Workspace `Blocked` → Deny all
   - Workspace `ReadOnly` → Allow only read-only tools (from normalizer config, not hardcoded list)
   - Workspace `Trusted` → continue to next layer

3. **Layer 3 — Policy file** (`.reasonance/permissions.toml`):
   - Parsed on first evaluate(), cached, invalidated on fs change
   - Per-tool decisions with optional pattern matching on args
   - Falls through to Layer 4 if tool not in file

4. **Layer 4 — Model config** (permission level from LLM config):
   - `yolo` → Allow (but Layer 1 still blocks destructive)
   - `locked` → Deny all non-read-only
   - `ask` → continue to Layer 5

5. **Layer 5 — Session memory** (`PermissionMemory::lookup()`):
   - Found `Allow` (Session/Project scope) → Allow
   - Found `Deny` (Session/Project scope) → Deny
   - Found `Allow` (Once scope) → Allow + consume
   - Not found → continue to Layer 6

6. **Layer 6 — Default**: Confirm (ask the user)

#### Permission mode behavior summary

| Mode | L1 Hardcoded | L2 Trust | L3 Policy | L4 Config | L5 Memory | L6 Default |
|------|-------------|----------|-----------|-----------|-----------|------------|
| **Yolo** | Deny destructive | Enforce trust | Apply | Allow all | Skip | Allow |
| **Ask** | Deny destructive | Enforce trust | Apply | Continue | Lookup | Confirm |
| **Locked** | Deny destructive | Enforce trust | Apply | Deny non-read | Skip | Deny |

### Backend: Transport integration

Replace inline trust checks (lines 100-135 of `transport/mod.rs`) with:

```rust
// Before spawning CLI:
let trust_level = trust_store.check_trust(cwd).level;
let ctx = PermissionContext {
    tool_name: "*".to_string(), // pre-flight check for overall mode
    tool_args: String::new(),
    provider: provider.clone(),
    permission_level: config.permission_level.clone(),
    trust_level: trust_level_str,
    project_root: cwd.map(|s| s.to_string()),
};

let decision = engine.evaluate(&ctx, &memory);
match decision {
    PermissionDecision::Deny { reason } => return Err(ReasonanceError::PermissionDenied { action: reason, tool: None }),
    PermissionDecision::Allow => { /* build --allowedTools from engine */ }
    PermissionDecision::Confirm => { /* build --allowedTools for ask mode */ }
}
```

Methods absorbed into engine:
- `build_permission_args_with_trust()` → engine decides if `--dangerously-skip-permissions` is needed
- `build_read_only_tools_args()` → engine uses normalizer config's `read_only_tools` for Layer 2
- `build_allowed_tools_args()` → engine builds list from memory + policy + config

#### Audit events

Every `evaluate()` publishes to EventBus `permission:decision`:

```rust
event_bus.publish(Event::new(
    "permission:decision",
    json!({
        "tool": ctx.tool_name,
        "decision": decision_str,
        "layer": deciding_layer,
        "session_id": session_id,
        "trust_level": ctx.trust_level,
        "permission_level": ctx.permission_level,
    }),
    "permission_engine",
));
```

### Backend: Layer 3 — Policy file

**File**: `.reasonance/permissions.toml` (project-level), `~/.config/reasonance/permissions.toml` (global)

**Format:**
```toml
[tools.Bash]
decision = "confirm"
patterns_deny = ["rm -rf", "DROP TABLE"]
patterns_allow = ["ls", "cat", "grep", "npm test"]

[tools.Write]
decision = "allow"

[tools.WebSearch]
decision = "deny"
```

**Parsing rules:**
- `decision`: "allow" | "deny" | "confirm"
- `patterns_deny`: if tool args contain any pattern → Deny (checked first)
- `patterns_allow`: if tool args contain any pattern → Allow
- If both patterns exist and neither matches → fall through to `decision`
- Project-level file takes priority over global (LayeredSettings pattern)

**Caching:** parsed on first access, stored in `PermissionEngine`. Invalidated when fs watcher detects change to the file. Re-parsed lazily on next `evaluate()`.

**Project scope persistence:** when a user approves a tool with scope "Project", the decision is appended to the project-level `permissions.toml`:
```rust
// In record_permission_decision when scope == Project:
policy_file.set_tool_decision(&tool_name, "allow");
policy_file.save()?;
engine.invalidate_policy_cache(); // force re-read
```

### Backend: Layer 5 — Session memory in evaluate

```rust
// Inside evaluate(), after Layer 4:
if let Some(stored) = memory.lookup(session_id, &ctx.tool_name) {
    match stored.action {
        Action::Allow => return PermissionDecision::Allow,
        Action::Deny => return PermissionDecision::Deny { reason: "denied by user".into() },
    }
}
// Layer 6: default Confirm
```

`lookup()` already handles Once scope consumption (returns + removes). Session scope persists in memory. Project scope persists in memory AND permissions.toml.

### Frontend: Per-tool approval UI

**Redesigned `PermissionRequestBlock.svelte`:**

When a `permission_denial` event arrives, render one block per denied tool:

```
┌─────────────────────────────────────────────────┐
│ Permission required                              │
│                                                  │
│ Write → src/lib/components/App.svelte            │
│   [Allow once] [Allow session] [Allow project] [Deny] │
│                                                  │
│ Bash → npm test                                  │
│   [Allow once] [Allow session] [Allow project] [Deny] │
│                                                  │
│ ⏱ Auto-deny in 4:32                             │
└─────────────────────────────────────────────────┘
```

Each tool shows:
- Tool name + arguments (what it wants to do)
- 4 action buttons with scope granularity
- Countdown timer (W2.2, default 5 min, configurable)

**On user action:**
1. Call `adapter.recordPermissionDecision(sessionId, toolName, action, scope)`
2. If all tools decided → trigger re-send
3. Re-send uses `--resume` if provider supports it, otherwise full replay with updated `--allowedTools`

**Eliminated:** `sessionApprovedTools` Set in ChatView.svelte — replaced entirely by `PermissionMemory` via adapter commands.

### Frontend: Adapter integration

New adapter methods (already exist as commands, need adapter wiring):
```typescript
recordPermissionDecision(sessionId: string, toolName: string, action: string, scope: string): Promise<void>
lookupPermissionDecision(sessionId: string, toolName: string): Promise<PermissionDecision | null>
listPermissionDecisions(sessionId: string): Promise<Array<[string, StoredDecision]>>
clearPermissionSession(sessionId: string): Promise<void>
waitForPermissionDecision(sessionId: string, toolName: string, timeoutSecs?: number): Promise<PermissionDecision>
```

All routed through `enqueue()` (batchable).

## Testing

### Rust tests

**Engine integration:**
- Yolo mode: Layer 1 blocks `rm -rf /` even with yolo (existing test, maintain)
- Yolo mode: Layer 1 allows normal Bash commands
- Ask mode: Layer 5 returns Allow for previously approved tool
- Ask mode: Layer 5 Once scope consumed after first use
- Locked mode: everything Deny except read-only tools
- Layer 3: valid TOML parsed, decisions applied
- Layer 3: absent file → skip (no error)
- Layer 3: patterns_deny blocks matching args
- Layer 3: patterns_allow allows matching args
- Layer 3: project scope decision writes to file
- Layer 3: cache invalidation on file change

**Transport integration:**
- `send()` calls `evaluate()` — no inline trust checks remain
- Deny from engine → `ReasonanceError::PermissionDenied`
- Allow from engine → CLI spawned with correct `--allowedTools`
- Audit events published on EventBus for every evaluate()

**Benchmark:**
- `evaluate()` < 1ms with all layers active

### Frontend tests (Vitest)

- PermissionRequestBlock renders per-tool blocks with tool name + args
- Click "Allow session" calls `recordPermissionDecision` with correct params
- All tools decided → triggers re-send
- No `sessionApprovedTools` Set in ChatView (removed)
- Adapter methods for permission commands are batchable (go through enqueue)

## Migration Strategy

### Phase 1 — Layer 3 implementation
- Add `PolicyFile` struct with TOML parsing, caching, invalidation
- Add `patterns_deny` / `patterns_allow` matching
- Integration with LayeredSettings for global + project levels
- Tests

### Phase 2 — Layer 5 integration
- Add `memory` parameter to `evaluate()`
- Implement lookup inside evaluate flow
- Project scope writes to permissions.toml
- Tests

### Phase 3 — Transport wiring
- Replace inline trust checks with `engine.evaluate()` call
- Absorb `build_permission_args_with_trust()` and friends into engine
- Add audit event publishing
- Tests + benchmark

### Phase 4 — Frontend
- Add permission adapter methods (wire existing commands through enqueue)
- Redesign PermissionRequestBlock for per-tool approval
- Remove `sessionApprovedTools` from ChatView
- Wire re-send with --resume or replay
- Tests

### Phase 5 — Validation
- Full test suite (Rust + frontend)
- Clippy, svelte-check, vite build
- Benchmark < 1ms

## Exit Criteria

- Permission engine is sole decision point (zero inline checks in transport)
- All 6 layers functional (hardcoded, trust, policy file, model config, session memory, default)
- Layer 1 hardcoded non-overridable even in yolo mode
- Layer 3 parses `.reasonance/permissions.toml` with pattern matching
- Layer 5 integrated in `evaluate()`, scope Project persists to file
- Destructive commands always denied regardless of any setting
- Untrusted workspaces restricted to read-only tools
- All decisions emit audit events on EventBus `permission:decision`
- Frontend per-tool approval with 4 scopes (once, session, project, deny)
- `sessionApprovedTools` eliminated from ChatView
- Benchmark: < 1ms per `evaluate()` invocation
- Inline trust checks in transport replaced by engine
