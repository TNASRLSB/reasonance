# Security Audit Report

**Date:** 2026-03-22
**Auditor persona:** Security engineer (penetration assessment)
**Judgment criterion:** Can a malicious input or extension compromise the user?
**Scope:** Reasonance IDE v0.5.0 (Svelte 5 + Tauri 2 + Rust)

---

## Executive Summary

Reasonance demonstrates solid security fundamentals: API keys are kept in environment variables (never in config files or frontend code), file system operations enforce project-root sandboxing, PTY spawn has a command allowlist, DOMPurify sanitizes all markdown-to-HTML paths, and CSP is configured without `unsafe-eval`. However, the audit identified **2 High**, **5 Medium**, and **4 Low** severity issues. The most impactful findings are: (1) workflow file operations (`load_workflow`, `save_workflow`, `delete_workflow`) bypass the project-root path validation used by `read_file`/`write_file`, allowing arbitrary file read/write/delete via crafted workflow paths; and (2) the Google Gemini API key is sent as a URL query parameter, exposing it in server logs, network intermediaries, and potentially browser history.

**Overall verdict:** A malicious project file (crafted `.reasonance/workflows/*.json`) or a compromised LLM response that manipulates workflow paths could write arbitrary files on disk. Immediate remediation is recommended for the two High-severity findings.

---

## Threat Model

### Attack Surface

| Surface | Entry Points | Trust Level |
|---------|-------------|-------------|
| **IPC (Tauri commands)** | 50+ commands exposed to webview frontend | Medium -- frontend JS can invoke any registered command |
| **File system** | `read_file`, `write_file`, `list_dir`, `grep_files`, workflow CRUD, config read/write | High -- user files at risk |
| **PTY / Shell** | `spawn_process`, `write_pty` | Critical -- arbitrary code execution potential |
| **Network** | LLM API calls (Anthropic, OpenAI, Google), updater endpoint | Medium -- credential exposure risk |
| **User input** | Chat messages, file paths, project names, workflow definitions | Medium -- injection vectors |
| **Dependencies** | npm packages, Rust crates | Low-Medium -- supply chain risk |
| **Markdown rendering** | LLM responses rendered as HTML via `{@html}` | Medium -- XSS if sanitization fails |

### Trust Boundaries

1. **Frontend (webview) <-> Backend (Rust):** All communication via Tauri IPC. Frontend is semi-trusted (CSP-constrained but could be influenced by XSS or malicious content).
2. **Backend <-> File system:** Path validation at `read_file`/`write_file` boundary, but not universally applied.
3. **Backend <-> External APIs:** API keys read from env vars, sent over HTTPS (except Google key in URL).
4. **Backend <-> Shell:** Command allowlist enforced at spawn, but PTY `write_pty` has no content filtering.

---

## Findings

### Critical (P0)

No critical findings.

### High (P1)

#### H1: Workflow commands bypass project-root path validation (arbitrary file read/write/delete)

**Location:** `src-tauri/src/commands/workflow.rs` (all commands), `src-tauri/src/workflow_store.rs:143-198`

**Description:** The `read_file` and `write_file` IPC commands correctly enforce project-root sandboxing via `validate_read_path()` / `validate_write_path()`. However, the workflow commands -- `load_workflow`, `save_workflow`, `delete_workflow`, `duplicate_workflow`, `save_to_global` -- accept arbitrary `file_path` strings and pass them directly to `std::fs::read_to_string`, `std::fs::write`, and `std::fs::remove_file` without any path validation.

**Exploit scenario:** A malicious frontend script (via XSS or compromised dependency) could invoke:
```js
invoke('save_workflow', { file_path: '/etc/cron.d/malicious', workflow: {...} })
invoke('delete_workflow', { file_path: '/home/user/.ssh/authorized_keys' })
invoke('load_workflow', { file_path: '/etc/shadow' })  // read arbitrary files
```

**Impact:** Arbitrary file write, read, and delete anywhere the process user has permission. This is a sandbox escape.

**Remediation:**
- Apply `validate_write_path()` / `validate_read_path()` to all workflow commands, or
- Restrict workflow paths to the project `.reasonance/workflows/` subdirectory with canonicalization checks.
- Apply the same treatment to `list_workflows` (currently accepts arbitrary `dir` parameter).

---

#### H2: Google Gemini API key exposed in URL query parameter

**Location:** `src-tauri/src/commands/llm.rs:127-129`

**Description:** The `call_google` function constructs the API URL as:
```rust
let url = format!(
    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
    m, api_key
);
```

The API key is embedded in the URL query string. This means the key will appear in:
- Server access logs on Google's side
- Any network proxy/CDN/WAF logs
- Potential crash reports or error messages that include the URL
- The `Referer` header if any redirects occur

**Impact:** API key leakage. While this is Google's standard API pattern, it is inferior to header-based authentication from a security perspective.

**Remediation:**
- Google Gemini API also supports `x-goog-api-key` header authentication. Switch to:
  ```rust
  .header("x-goog-api-key", api_key)
  ```
  and remove the `key=` query parameter.

---

### Medium (P2)

#### M1: `list_dir` and `grep_files` commands lack project-root path validation

**Location:** `src-tauri/src/commands/fs.rs:135`, `src-tauri/src/commands/fs.rs:188`

**Description:** While `read_file` and `write_file` enforce project-root validation via `validate_read_path()`, the `list_dir` and `grep_files` commands accept arbitrary paths without any validation. This allows the frontend to enumerate directory contents and search file contents anywhere on disk.

**Exploit scenario:**
```js
invoke('list_dir', { path: '/home/user/.ssh', respect_gitignore: false })
invoke('grep_files', { path: '/etc', pattern: 'password', respect_gitignore: false })
```

**Impact:** Information disclosure -- directory listing and content search across the entire filesystem.

**Remediation:** Apply the same `validate_read_path()` check (or a directory-level equivalent) to `list_dir` and `grep_files`. At minimum, require paths to be within the project root or the Reasonance config directory.

---

#### M2: `start_watching` accepts arbitrary paths without validation

**Location:** `src-tauri/src/commands/fs.rs:228-234`

**Description:** The `start_watching` command passes the user-supplied path directly to the filesystem watcher without project-root validation. This could be used to monitor sensitive directories for changes.

**Impact:** Information disclosure via filesystem event monitoring outside project scope.

**Remediation:** Validate the path is within the project root before starting the watcher.

---

#### M3: DOMPurify used with default configuration (no hardened profile)

**Location:**
- `src/lib/components/MarkdownPreview.svelte:23`
- `src/lib/components/ResponsePanel.svelte:34`
- `src/lib/components/chat/TextBlock.svelte:10`

**Description:** All three markdown rendering components use `DOMPurify.sanitize()` with default settings. While DOMPurify defaults are reasonable, they allow elements like `<form>`, `<input>`, `<details>`, `<summary>`, and `<svg>` which could be used for UI redressing or phishing in LLM responses.

The default config also allows `<a>` tags with arbitrary `href` attributes, potentially enabling `javascript:` protocol links in some edge cases (though DOMPurify default blocks this, it is worth being explicit).

**Impact:** Potential UI redressing or phishing via crafted LLM responses rendered as HTML.

**Remediation:** Use a hardened DOMPurify config:
```js
DOMPurify.sanitize(html, {
  ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'a', 'code', 'pre', 'ul', 'ol', 'li',
                 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'table', 'thead',
                 'tbody', 'tr', 'th', 'td', 'hr', 'img', 'span', 'del', 'sup', 'sub'],
  ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class', 'id'],
  ALLOW_DATA_ATTR: false,
})
```

---

#### M4: `write_config` command writes to fixed config path without content validation

**Location:** `src-tauri/src/commands/config.rs:13-19`

**Description:** The `write_config` command accepts arbitrary string content and writes it to the Reasonance config file (`~/.config/reasonance/llms.toml`). While the path is fixed (not user-controlled), the content is not validated as valid TOML. A malicious frontend could write corrupted or maliciously crafted config content.

More concerning: the config file is later parsed by `is_allowed_command()` in `pty.rs` to determine which commands are permitted for PTY spawn. By writing a config with a malicious `command` field (e.g., `command = "rm"`), an attacker could add arbitrary binaries to the PTY command allowlist.

**Exploit chain:**
1. Call `write_config` with TOML containing `[[llm]] name = "exploit" type = "cli" command = "rm"`
2. Call `spawn_process` with `command = "rm"` and args `["-rf", "/home/user/important"]`

**Impact:** Privilege escalation from config write to arbitrary command execution.

**Remediation:**
- Validate the TOML content parses correctly before writing.
- Validate that `command` values in LLM config entries match a pattern (e.g., no absolute paths to system utilities, or only allow known LLM binaries).
- Consider separating the "allowed commands" logic from user-editable config.

---

#### M5: CSP allows `unsafe-inline` for styles

**Location:** `src-tauri/tauri.conf.json:25`

**Description:** The CSP includes `style-src 'self' 'unsafe-inline'`. While `unsafe-inline` for styles is less dangerous than for scripts, it can enable CSS-based data exfiltration attacks (e.g., CSS attribute selectors that load external resources based on element content).

The CSP is otherwise well-configured: `script-src 'self'` (no `unsafe-eval`), `connect-src` limited to known API endpoints, no `unsafe-inline` for scripts.

**Impact:** Low-medium. CSS injection could potentially be used for limited data exfiltration if combined with another vulnerability.

**Remediation:** If Svelte's runtime styling permits it, use CSP nonces or hashes instead of `unsafe-inline` for styles. This may require build tooling changes.

---

### Low (P3)

#### L1: `ResourceNode.svelte` uses `{@html}` with hardcoded HTML entities (non-exploitable)

**Location:** `src/lib/components/swarm/ResourceNode.svelte:27`

**Description:** `{@html kindIcons[kind] || '&#128196;'}` -- the `kind` prop is used to index into a hardcoded `kindIcons` dictionary. The fallback is a safe HTML entity. Since `kind` values come from workflow definitions (which could be attacker-controlled via crafted workflow JSON), a non-matching `kind` falls through to the safe default. The dictionary values are all safe HTML entities.

**Impact:** No current exploit, but the pattern of using `{@html}` with dictionary lookups on user-controlled keys is fragile. If the dictionary were later expanded with dynamic values, it could become exploitable.

**Remediation:** Consider using Unicode characters directly instead of `{@html}` with HTML entities, or validate `kind` against the known set.

---

#### L2: Error display in `app.html` uses `textContent` (safe) but exposes stack traces

**Location:** `src/app.html:12-24`

**Description:** The global error handler creates a `<pre>` element and sets `d.textContent = msg`, which is safe against XSS. However, it displays full stack traces including file paths and line numbers to the user. In a release build, this could leak internal file structure information.

**Impact:** Information disclosure (internal paths) in error conditions.

**Remediation:** Conditionally disable detailed error display in production builds, or limit the information shown.

---

#### L3: `get_env_var` allowlist includes `PATH` and `HOME`

**Location:** `src-tauri/src/commands/system.rs:52-70`

**Description:** The env var allowlist (SEC-04) includes `PATH`, `HOME`, `USER`, `SHELL`, `TERM`, and `XDG_CONFIG_HOME`. While these are not secrets, exposing `PATH` reveals installed software locations and `HOME` reveals the username. The allowlist appropriately blocks arbitrary env var access.

**Impact:** Minor information disclosure.

**Remediation:** Consider whether `PATH` truly needs to be exposed to the frontend. If only needed for backend discovery, remove it from the allowlist.

---

#### L4: Supply chain -- unmaintained GTK3 Rust bindings

**Description:** `cargo audit` reports 12 warnings for unmaintained GTK3-related crates (atk, gdk, gtk, and their `-sys` variants, all RUSTSEC-2024-04xx). These are transitive dependencies of Tauri's Linux rendering backend (wry -> gtk). Additionally, `proc-macro-error` v1.0.4 is unmaintained (RUSTSEC-2024-0370).

npm audit reports 3 low-severity issues in the `cookie` package (used transitively by `@sveltejs/kit`), related to out-of-bounds character acceptance in cookie names/paths/domains.

**Impact:** No known exploitable vulnerabilities currently, but unmaintained dependencies will not receive security patches.

**Remediation:**
- Monitor Tauri releases for GTK4 migration (in progress upstream).
- For npm: the `cookie` issue is low severity and the fix requires a breaking `@sveltejs/kit` downgrade; monitor for a compatible patch release.

---

## Vulnerability Register

| # | Severity | Vector | Location | Description | Remediation |
|---|----------|--------|----------|-------------|-------------|
| H1 | High | IPC / FS | `src-tauri/src/commands/workflow.rs` (all commands), `src-tauri/src/workflow_store.rs:143-198` | Workflow load/save/delete bypass project-root path validation, enabling arbitrary file read/write/delete | Apply `validate_read_path`/`validate_write_path` to all workflow file operations |
| H2 | High | Network | `src-tauri/src/commands/llm.rs:127-129` | Google Gemini API key sent as URL query parameter | Use `x-goog-api-key` header instead of `?key=` parameter |
| M1 | Medium | IPC / FS | `src-tauri/src/commands/fs.rs:135,188` | `list_dir` and `grep_files` accept arbitrary paths without project-root validation | Add path validation matching `read_file`/`write_file` |
| M2 | Medium | IPC / FS | `src-tauri/src/commands/fs.rs:228-234` | `start_watching` accepts arbitrary paths without validation | Validate path is within project root |
| M3 | Medium | XSS | `src/lib/components/MarkdownPreview.svelte:23`, `ResponsePanel.svelte:34`, `chat/TextBlock.svelte:10` | DOMPurify used with default config; allows forms, inputs, SVG | Use hardened allowlist configuration |
| M4 | Medium | IPC / Config | `src-tauri/src/commands/config.rs:13-19`, `src-tauri/src/commands/pty.rs:9-51` | Config write + PTY allowlist = privilege escalation chain | Validate TOML content; separate command allowlist from user config |
| M5 | Medium | CSP | `src-tauri/tauri.conf.json:25` | `style-src 'self' 'unsafe-inline'` allows CSS injection | Use nonces or hashes if feasible |
| L1 | Low | XSS | `src/lib/components/swarm/ResourceNode.svelte:27` | `{@html}` with dictionary lookup on user-controlled key (currently safe) | Use Unicode chars instead of HTML entities |
| L2 | Low | InfoDisc | `src/app.html:12-24` | Stack traces with file paths shown on error | Disable in production builds |
| L3 | Low | InfoDisc | `src-tauri/src/commands/system.rs:52-70` | `PATH` and `HOME` exposed via `get_env_var` allowlist | Remove non-essential env vars from allowlist |
| L4 | Low | Supply chain | `Cargo.toml` (transitive), `package.json` (transitive) | 12 unmaintained Rust GTK3 crates; 3 low-severity npm `cookie` issues | Monitor upstream; no immediate action required |

---

## Positive Security Observations

The following security measures are well-implemented and deserve acknowledgment:

1. **API keys never reach frontend code.** The `api_key_env` field stores only the env var *name*; the actual key is read server-side in Rust (`std::env::var`) and sent directly to the API. Keys are not logged, not included in IPC responses, and not stored in config files.

2. **PTY command allowlist** (`src-tauri/src/commands/pty.rs:6-51`): Only known shells and configured LLM commands can be spawned. Arbitrary commands like `rm`, `curl`, `python3` are rejected. The allowlist checks both the full path and the binary basename.

3. **File I/O sandboxing** (for `read_file`/`write_file`): Path canonicalization with `std::fs::canonicalize` prevents symlink-based traversal. Reads allow project root + config dir; writes require project root only.

4. **`open_external` URL scheme validation** (`src-tauri/src/commands/system.rs:42-48`, SEC-06): Only `http://` and `https://` schemes are allowed, preventing `file://` and custom scheme abuse.

5. **Environment variable access control** (SEC-04): Hard-coded allowlist prevents arbitrary env var enumeration.

6. **CSP script-src** lacks `unsafe-eval` and `unsafe-inline`, preventing most script injection attacks.

7. **`withGlobalTauri: false`**: The Tauri API is not exposed globally on `window.__TAURI__`, reducing the attack surface from browser console or injected scripts.

8. **Updater uses public key verification** (`tauri.conf.json:43`): Update packages are signature-verified.

---

## Recommended Priority Actions

1. **Immediate (H1):** Add path validation to all workflow commands. This is the most exploitable finding.
2. **Immediate (H2):** Switch Google API to header-based auth.
3. **Short-term (M1, M2):** Extend path validation to `list_dir`, `grep_files`, and `start_watching`.
4. **Short-term (M4):** Add TOML validation to `write_config` and decouple PTY allowlist from user-editable config.
5. **Medium-term (M3):** Harden DOMPurify configuration across all rendering components.
6. **Medium-term (M5):** Evaluate CSP nonce/hash feasibility for style-src.
