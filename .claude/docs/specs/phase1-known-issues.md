# Phase 1 — Known Issues

Issues identified during final code review. Non-blocking, to be addressed in subsequent phases.

---

### I1: `split_operator` does not handle escaped quotes

**Location:** `src-tauri/src/normalizer/rules_engine.rs` — `split_operator()`

**Issue:** If a TOML expression contains `field == "value with \" escaped"`, the `in_quotes` toggle will incorrectly treat the backslash-escaped quote as a string terminator. Unlikely with current TOML rules but could cause subtle bugs if expressions grow more complex.

**Fix:** Add backslash-escape tracking in the `split_operator` loop (`if bytes[i] == b'\\' { i += 1; continue; }`).

**When:** Phase 6 (self-heal generates expressions) or when adding provider TOMLs with complex string literals.

---

### I2: `parse_content` loses structure for multi-block text

**Location:** `src-tauri/src/normalizer/content_parser.rs:6-13`, `src-tauri/src/normalizer/pipeline.rs:125-134`

**Issue:** When text contains mixed content (e.g., prose + code block + prose = 3 blocks), `parse_content()` falls back to returning the original text as `EventContent::Text` instead of preserving parsed blocks. The pipeline's `enrich_content()` calls `parse_content` (not `parse_content_blocks`), so mixed-content responses lose their structure.

**Fix:** Either return a new `EventContent::Blocks(Vec<EventContent>)` variant, or have the pipeline call `parse_content_blocks` and emit multiple `AgentEvent`s per input event.

**When:** Phase 5 (Enrichment — DiffBlock, ActionableMessage).

---

### I3: `severity` mapping uses literal values, undocumented

**Location:** `src-tauri/src/normalizer/pipeline.rs:94`

**Issue:** The `severity` mapping is treated as a literal string (`"recoverable"`, `"fatal"`), while all other mappings use `resolve_path` to extract values from JSON. This is intentional (severity is a fixed rule property, not a JSON field) but undocumented — easy to confuse with a JSON path.

**Fix:** Add a comment in `pipeline.rs` and in the TOML rules documentation clarifying that `severity` is a literal, not a path.

**When:** Phase 2 (when documenting the Transport API) or next touch of these files.
