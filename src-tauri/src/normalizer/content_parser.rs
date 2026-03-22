use crate::agent_event::{DiffHunk, EventContent};

/// Parses a single text string into the most appropriate EventContent type.
/// For simple text without code blocks or diffs, returns EventContent::Text.
pub fn parse_content(text: &str) -> EventContent {
    let blocks = parse_content_blocks(text);
    if blocks.len() == 1 {
        blocks.into_iter().next().unwrap()
    } else {
        // Multiple blocks: return as text (the caller handles block-level rendering)
        EventContent::Text { value: text.to_string() }
    }
}

/// Parses text into a sequence of typed content blocks.
/// Detects: code fences (```lang ... ```), unified diffs (--- +++ @@).
pub fn parse_content_blocks(text: &str) -> Vec<EventContent> {
    let mut blocks = Vec::new();
    let mut current_text = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Detect code fence
        if line.starts_with("```") {
            // Flush accumulated text
            if !current_text.is_empty() {
                blocks.push(EventContent::Text { value: current_text.trim_end().to_string() });
                current_text.clear();
            }

            let language = line[3..].trim().to_string();
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            blocks.push(EventContent::Code {
                language,
                source: code_lines.join("\n"),
            });
            i += 1; // skip closing ```
            continue;
        }

        // Detect unified diff (needs ---, +++, and @@ markers)
        if line.starts_with("--- ") && i + 2 < lines.len()
            && lines[i + 1].starts_with("+++ ")
            && lines[i + 2].starts_with("@@ ")
        {
            // Flush accumulated text
            if !current_text.is_empty() {
                blocks.push(EventContent::Text { value: current_text.trim_end().to_string() });
                current_text.clear();
            }

            let file_path = extract_diff_path(lines[i + 1]);
            i += 2; // skip --- and +++

            let mut hunks = Vec::new();
            while i < lines.len() && lines[i].starts_with("@@ ") {
                let (hunk, next_i) = parse_diff_hunk(&lines, i);
                hunks.push(hunk);
                i = next_i;
            }

            if !hunks.is_empty() {
                blocks.push(EventContent::Diff { file_path, hunks });
            }
            continue;
        }

        // Regular text line
        current_text.push_str(line);
        current_text.push('\n');
        i += 1;
    }

    // Flush remaining text
    if !current_text.is_empty() {
        blocks.push(EventContent::Text { value: current_text.trim_end().to_string() });
    }

    if blocks.is_empty() {
        blocks.push(EventContent::Text { value: String::new() });
    }

    blocks
}

/// Extracts the file path from a +++ line (e.g., "+++ b/src/main.rs" → "src/main.rs").
fn extract_diff_path(line: &str) -> String {
    let path = line.trim_start_matches("+++ ").trim_start_matches("b/");
    path.to_string()
}

/// Parses a single diff hunk starting at the @@ line.
/// Returns the hunk and the index of the next line after the hunk.
fn parse_diff_hunk(lines: &[&str], start: usize) -> (DiffHunk, usize) {
    let header = lines[start];
    let (old_start, new_start) = parse_hunk_header(header);

    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("@@ ") || line.starts_with("--- ") || line.starts_with("+++ ") {
            break;
        }
        if let Some(stripped) = line.strip_prefix('-') {
            old_lines.push(stripped.to_string());
        } else if let Some(stripped) = line.strip_prefix('+') {
            new_lines.push(stripped.to_string());
        }
        i += 1;
    }

    (DiffHunk { old_start, new_start, old_lines, new_lines }, i)
}

/// Parses "@@ -1,3 +1,3 @@" into (old_start, new_start).
fn parse_hunk_header(header: &str) -> (u32, u32) {
    let parts: Vec<&str> = header.split_whitespace().collect();
    let old_start = parts.get(1)
        .and_then(|s| s.trim_start_matches('-').split(',').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let new_start = parts.get(2)
        .and_then(|s| s.trim_start_matches('+').split(',').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    (old_start, new_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let result = parse_content("Hello, this is plain text.");
        assert!(matches!(result, EventContent::Text { .. }));
    }

    #[test]
    fn test_code_block_rust() {
        let input = "Here is some code:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\nEnd.";
        let blocks = parse_content_blocks(input);
        assert_eq!(blocks.len(), 3); // text, code, text
        assert!(matches!(&blocks[1], EventContent::Code { language, .. } if language == "rust"));
    }

    #[test]
    fn test_code_block_no_language() {
        let input = "```\nsome code\n```";
        let blocks = parse_content_blocks(input);
        assert!(matches!(&blocks[0], EventContent::Code { language, .. } if language.is_empty()));
    }

    #[test]
    fn test_unified_diff_detection() {
        let input = "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@\n-old line\n+new line\n context\n";
        let blocks = parse_content_blocks(input);
        assert!(matches!(&blocks[0], EventContent::Diff { .. }));
    }

    #[test]
    fn test_diff_hunk_parsing() {
        let input = "--- a/test.rs\n+++ b/test.rs\n@@ -1,2 +1,2 @@\n-removed\n+added\n kept\n";
        let blocks = parse_content_blocks(input);
        if let EventContent::Diff { hunks, file_path, .. } = &blocks[0] {
            assert_eq!(file_path, "test.rs");
            assert_eq!(hunks.len(), 1);
            assert_eq!(hunks[0].old_lines, vec!["removed"]);
            assert_eq!(hunks[0].new_lines, vec!["added"]);
        } else {
            panic!("Expected Diff content");
        }
    }

    #[test]
    fn test_pseudo_diff_falls_back_to_text() {
        let input = "--- Some heading ---\n+++ Another heading +++\nJust text, no @@ markers";
        let blocks = parse_content_blocks(input);
        assert!(matches!(&blocks[0], EventContent::Text { .. }));
    }

    #[test]
    fn test_file_ref_not_detected_in_plain_text() {
        let result = parse_content("I edited src/main.rs");
        assert!(matches!(result, EventContent::Text { .. }));
    }

    #[test]
    fn test_mixed_content() {
        let input = "Here is the fix:\n```python\nprint('hello')\n```\nDone.";
        let blocks = parse_content_blocks(input);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], EventContent::Text { .. }));
        assert!(matches!(&blocks[1], EventContent::Code { language, .. } if language == "python"));
        assert!(matches!(&blocks[2], EventContent::Text { .. }));
    }
}
