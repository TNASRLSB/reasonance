/// Built-in defaults — the base layer that always exists.
/// These provide sensible out-of-the-box values for all settings.
pub fn builtin_defaults() -> toml::Value {
    toml::from_str(
        r#"
[editor]
font_size = 14
tab_size = 2
font_family = "Atkinson Hyperlegible Mono"

[terminal]
font_size = 13

[analytics]
enabled = true

[filetree]
auto_fold = false
show_git_status = true
"#,
    )
    .expect("builtin defaults must be valid TOML")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_defaults_are_valid() {
        let val = builtin_defaults();
        assert!(val.is_table());
        assert!(val.get("editor").is_some());
        assert!(val.get("terminal").is_some());
        assert!(val.get("analytics").is_some());
        assert!(val.get("filetree").is_some());
    }
}
