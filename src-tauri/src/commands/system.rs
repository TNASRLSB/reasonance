use std::process::Command;

#[derive(serde::Serialize)]
pub struct DiscoveredLlm {
    pub name: String,
    pub command: String,
    pub found: bool,
}

#[tauri::command]
pub fn discover_llms() -> Vec<DiscoveredLlm> {
    let candidates = vec![
        ("Claude", "claude"),
        ("Gemini", "gemini"),
        ("Aider", "aider"),
        ("GitHub Copilot", "github-copilot-cli"),
        ("Ollama", "ollama"),
        ("Open Interpreter", "interpreter"),
    ];

    candidates
        .into_iter()
        .map(|(name, cmd)| {
            let found = if cfg!(target_os = "windows") {
                Command::new("where").arg(cmd).output()
            } else {
                Command::new("which").arg(cmd).output()
            }
            .map(|o| o.status.success())
            .unwrap_or(false);

            DiscoveredLlm {
                name: name.to_string(),
                command: cmd.to_string(),
                found,
            }
        })
        .collect()
}

#[tauri::command]
pub fn open_external(path: String) -> Result<(), String> {
    // SEC-06: only allow http:// and https:// schemes to prevent file:// and app:// abuse
    if !path.starts_with("https://") && !path.starts_with("http://") {
        return Err(format!("Rejected URL with disallowed scheme: {}", path));
    }
    open::that(&path).map_err(|e| e.to_string())
}

/// SEC-04: restricted to a hard-coded allowlist of known LLM-related env var names.
/// Arbitrary env var access is not permitted to prevent credential or path leakage.
const ENV_VAR_ALLOWLIST: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "GEMINI_API_KEY",
    "GROQ_API_KEY",
    "MISTRAL_API_KEY",
    "TOGETHER_API_KEY",
    "DEEPSEEK_API_KEY",
    "OPENROUTER_API_KEY",
    "HF_TOKEN",
    "OLLAMA_HOST",
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "TERM",
    "XDG_CONFIG_HOME",
];

#[tauri::command]
pub fn get_env_var(name: String) -> Result<Option<String>, String> {
    if !ENV_VAR_ALLOWLIST.contains(&name.as_str()) {
        return Err(format!("Environment variable '{}' is not in the allowed list", name));
    }
    Ok(std::env::var(&name).ok())
}

#[tauri::command]
pub fn get_system_colors() -> Result<std::collections::HashMap<String, String>, String> {
    let mut colors = std::collections::HashMap::new();

    // Try reading KDE globals
    if let Some(config_dir) = dirs::config_dir() {
        let kde_globals = config_dir.join("kdeglobals");
        if kde_globals.exists() {
            if let Ok(content) = std::fs::read_to_string(&kde_globals) {
                parse_kde_colors(&content, &mut colors);
                return Ok(colors);
            }
        }
    }

    // Fallback: return empty (frontend uses its own defaults)
    Ok(colors)
}

fn parse_kde_colors(content: &str, colors: &mut std::collections::HashMap<String, String>) {
    let mut section = String::new();
    let mut bg_r: Option<u8> = None;
    let mut bg_g: Option<u8> = None;
    let mut bg_b: Option<u8> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match (section.as_str(), key) {
                ("Colors:Window", "BackgroundNormal") => {
                    let hex = rgb_to_hex(value);
                    colors.insert("bg".into(), hex);
                    // Parse RGB values for luminance calculation
                    let parts: Vec<u8> = value.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                    if parts.len() >= 3 {
                        bg_r = Some(parts[0]);
                        bg_g = Some(parts[1]);
                        bg_b = Some(parts[2]);
                    }
                }
                ("Colors:Window", "ForegroundNormal") => {
                    colors.insert("fg".into(), rgb_to_hex(value));
                }
                ("Colors:Selection", "BackgroundNormal") => {
                    colors.insert("accent".into(), rgb_to_hex(value));
                }
                ("Colors:View", "BackgroundNormal") => {
                    colors.insert("bg_secondary".into(), rgb_to_hex(value));
                }
                ("General", "ColorScheme") => {
                    colors.insert("scheme".into(), value.to_string());
                }
                _ => {}
            }
        }
    }

    // Determine is_dark from background luminance (relative luminance < 0.5 = dark)
    if let (Some(r), Some(g), Some(b)) = (bg_r, bg_g, bg_b) {
        let luminance = 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64);
        colors.insert("is_dark".into(), if luminance < 128.0 { "true" } else { "false" }.into());
    }
}

fn rgb_to_hex(rgb: &str) -> String {
    let parts: Vec<u8> = rgb
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if parts.len() >= 3 {
        format!("#{:02x}{:02x}{:02x}", parts[0], parts[1], parts[2])
    } else {
        rgb.to_string()
    }
}
