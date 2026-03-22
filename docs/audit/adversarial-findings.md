# Adversarial Testing Report

**Date:** 2026-03-22
**Status:** Code-level analysis — live adversarial testing requires human execution

> **Note:** These findings are derived from code-level analysis of input handling, error paths, and edge case behavior. Items marked 🔍 need live verification with a running app.

---

## Input Extremes

| Test | Expected | Actual (from code) | Severity |
|------|----------|-------------------|----------|
| 1MB text in ChatInput | Graceful handling, maybe truncation | No input size limit in ChatInput.svelte; entire string sent to backend via IPC | ⚠️ Medium |
| Long SearchPalette query | Debounce, no hang | `buildFileList` is recursive with no depth/count limit; matching is synchronous filter | ⚠️ Medium |
| `<script>alert(1)</script>` in chat | Sanitized | DOMPurify sanitizes `{@html}` output; default config (allows `<form>`, `<input>`, `<svg>`) | ⚠️ Medium |
| `'; DROP TABLE` in inputs | No SQL injection | Backend uses parameterized queries via `rusqlite` — safe | ✅ Low |
| `../../etc/passwd` in file paths | Blocked | `read_file`/`write_file` validate against project root; BUT `list_dir`, `grep_files`, `load_workflow` DO NOT | ❌ High |
| Very long filename (255 chars) | Display truncated | `text-overflow: ellipsis` in FileTree — handled | ✅ Low |
| Unicode/emoji in filenames | Display correctly | UTF-8 throughout — likely OK | ✅ Low |
| Null bytes in input | Rejected | Rust strings are valid UTF-8 by type system — null bytes cause serde error | ✅ Low |

---

## Rapid Interactions

| Test | Expected | Actual (from code) | Severity |
|------|----------|-------------------|----------|
| Double-click send button | Debounce, single send | No debounce in ChatInput submit handler | ⚠️ Medium |
| Rapid tab switching | Smooth transition | Each switch destroys+recreates CodeMirror; no debounce | ⚠️ Medium |
| Switch tabs during streaming | Stream continues, display updates | Agent events tied to session, not tab — likely safe but 🔍 verify | ⚠️ Low |
| Resize window during stream | Layout adjusts | Terminal has RAF resize coalescing ✅; other panels use CSS flexbox ✅ | ✅ Low |
| Open/close settings rapidly | No state corruption | Settings is a modal overlay — open/close is simple boolean | ✅ Low |
| Rapid view mode toggles | Smooth | ViewModeToggle sets a single store value — race-free | ✅ Low |
| Multiple simultaneous agent sends | Queue or reject | No guard — multiple concurrent sends possible, events may interleave | ❌ High |

---

## File System Edge Cases

| Test | Expected | Actual (from code) | Severity |
|------|----------|-------------------|----------|
| 50MB file open | Warn or refuse | `fs::read_to_string` loads entirely into memory, sends full content to frontend via IPC | ❌ High |
| Binary file open | Show warning | `console.error` only — no UI feedback to user | ⚠️ Medium |
| Symlink loop | Detect and skip | `SearchPalette.buildFileList` recursive traversal with no cycle detection | ❌ High |
| Deeply nested dirs (20+ levels) | Render tree | FileTree recursive rendering — no depth limit but likely works, 🔍 verify performance | ⚠️ Low |
| Dotfiles (`.env`, `.git`) | Show or filter | FileTree filters `.git` but shows `.env` — 🔍 verify | ⚠️ Low |
| File modified externally | Detect and reload | `start_watching` uses file watcher but no path validation on watched directory | ⚠️ Medium |
| Empty directory | Show empty state | WelcomeScreen shown when no project loaded ✅; empty dir shows empty FileTree ✅ | ✅ Low |
| Read-only file | Show indicator | No read-only detection — user edits then save fails | ⚠️ Medium |
| File deleted while open | Handle gracefully | 🔍 Likely shows stale content — no "file deleted" detection in editor | ⚠️ Medium |

---

## Network & Provider Edge Cases

| Test | Expected | Actual (from code) | Severity |
|------|----------|-------------------|----------|
| Network offline during stream | Error + retry option | Transport layer has retry logic with exponential backoff ✅ | ✅ Low |
| Invalid API key | Clear error message | Backend returns error; 🔍 verify frontend shows it clearly | ⚠️ Low |
| Switch provider mid-stream | Cancel current + start new | 🔍 Requires live testing — potential for orphaned sessions | ⚠️ Medium |
| 5+ concurrent sessions | Handle without crash | Session manager uses Arc<Mutex>; 🔍 verify no deadlocks with concurrent access | ⚠️ Medium |
| Provider rate limiting | Show rate limit message | 🔍 Depends on provider error parsing — verify | ⚠️ Low |
| Response with malicious markdown | Sanitize | DOMPurify active but default config — `<form>` elements possible | ⚠️ Medium |

---

## Concurrency & State Races

| Test | Expected | Actual (from code) | Severity |
|------|----------|-------------------|----------|
| Two tabs editing same file | Conflict detection | No conflict detection — last-write-wins, data loss possible | ❌ High |
| File save during agent write | Atomic or queued | Backend uses `tokio::fs::write` — not atomic (no temp+rename) | ⚠️ Medium |
| Multiple rapid config saves | Last value wins cleanly | Config store has no debounce; rapid writes to TOML possible | ⚠️ Low |
| Agent event during component unmount | No crash | Svelte 5 `$effect` cleanup should handle; 🔍 verify no stale state updates | ⚠️ Low |

---

## Summary

| Category | Critical/High | Medium | Low |
|----------|--------------|--------|-----|
| Input Extremes | 1 | 3 | 4 |
| Rapid Interactions | 1 | 2 | 4 |
| File System | 2 | 4 | 2 |
| Network/Provider | 0 | 3 | 3 |
| Concurrency | 1 | 2 | 2 |
| **Total** | **5** | **14** | **15** |

### Top 5 Adversarial Risks

1. **Path traversal via unvalidated commands** (`list_dir`, `grep_files`, `load_workflow`) — attacker-controlled paths can read/write/delete anywhere
2. **50MB file opens without guard** — denial of service via memory exhaustion
3. **Symlink loop crashes SearchPalette** — infinite recursion with no cycle detection
4. **Multiple simultaneous agent sends** — event interleaving corrupts chat state
5. **Two tabs editing same file** — last-write-wins causes silent data loss
