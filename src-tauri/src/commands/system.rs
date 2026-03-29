use crate::error::ReasonanceError;
use log::{debug, error, info, warn};
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

#[derive(serde::Serialize)]
pub struct DiscoveredLlm {
    pub name: String,
    pub command: String,
    pub found: bool,
}

/// Build an extended PATH that includes common CLI installation directories
/// not always present in the default PATH (npm global, cargo, pip, etc.).
fn build_extended_path() -> OsString {
    let current_path = env::var_os("PATH").unwrap_or_default();
    let mut extra_dirs: Vec<PathBuf> = Vec::new();

    if let Some(home) = dirs::home_dir() {
        // npm global (custom prefix)
        extra_dirs.push(home.join(".npm-global").join("bin"));
        // npm global (default on macOS/Linux)
        extra_dirs.push(home.join(".npm").join("bin"));
        // cargo install
        extra_dirs.push(home.join(".cargo").join("bin"));
        // pip / pipx
        extra_dirs.push(home.join(".local").join("bin"));
        // pnpm global
        extra_dirs.push(home.join(".local").join("share").join("pnpm"));
        // bun global
        extra_dirs.push(home.join(".bun").join("bin"));
        // volta (Node version manager)
        extra_dirs.push(home.join(".volta").join("bin"));
        // nvm default (common symlink)
        extra_dirs.push(home.join(".nvm").join("current").join("bin"));
        // fnm
        extra_dirs.push(
            home.join(".local")
                .join("share")
                .join("fnm")
                .join("aliases")
                .join("default")
                .join("bin"),
        );
    }

    #[cfg(target_os = "windows")]
    if let Some(appdata) = env::var_os("APPDATA") {
        // npm global on Windows
        extra_dirs.push(PathBuf::from(&appdata).join("npm"));
    }
    #[cfg(target_os = "windows")]
    if let Some(localappdata) = env::var_os("LOCALAPPDATA") {
        extra_dirs.push(
            PathBuf::from(&localappdata)
                .join("Programs")
                .join("Python")
                .join("Scripts"),
        );
    }

    // Also try `npm prefix -g` for non-standard npm global locations
    if let Ok(output) = Command::new("npm").args(["prefix", "-g"]).output() {
        if output.status.success() {
            if let Ok(prefix) = String::from_utf8(output.stdout) {
                let npm_bin = PathBuf::from(prefix.trim()).join("bin");
                if !extra_dirs.contains(&npm_bin) {
                    extra_dirs.push(npm_bin);
                }
            }
        }
    }

    // Filter to only existing directories, then prepend to PATH
    let existing: Vec<PathBuf> = extra_dirs.into_iter().filter(|p| p.is_dir()).collect();

    if existing.is_empty() {
        return current_path;
    }

    let mut paths: Vec<PathBuf> = existing;
    // Append original PATH entries
    paths.extend(env::split_paths(&current_path));
    env::join_paths(paths).unwrap_or(current_path)
}

/// Find a binary in the extended PATH, returning its full path if found.
fn find_binary(cmd: &str, extended_path: &OsString) -> Option<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(cmd)
            .env("PATH", extended_path)
            .output()
    } else {
        Command::new("which")
            .arg(cmd)
            .env("PATH", extended_path)
            .output()
    };

    match output {
        Ok(o) if o.status.success() => {
            // Return the first line (full path to binary)
            String::from_utf8(o.stdout)
                .ok()
                .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
                .filter(|s| !s.is_empty())
        }
        _ => None,
    }
}

#[tauri::command]
pub fn discover_llms() -> Vec<DiscoveredLlm> {
    info!("cmd::discover_llms called");
    let candidates = vec![
        ("Claude", "claude"),
        ("Gemini", "gemini"),
        ("Aider", "aider"),
        ("GitHub Copilot", "github-copilot-cli"),
        ("Ollama", "ollama"),
        ("Open Interpreter", "interpreter"),
        ("Kimi", "kimi"),
        ("Qwen", "qwen"),
        ("Codex", "codex"),
    ];

    let extended_path = build_extended_path();
    debug!("cmd::discover_llms using extended PATH");

    let result: Vec<DiscoveredLlm> = candidates
        .into_iter()
        .map(|(name, cmd)| {
            match find_binary(cmd, &extended_path) {
                Some(full_path) => {
                    info!("cmd::discover_llms found {} at {}", cmd, full_path);
                    DiscoveredLlm {
                        name: name.to_string(),
                        // Use full path so spawn works even if PATH is limited
                        command: full_path,
                        found: true,
                    }
                }
                None => DiscoveredLlm {
                    name: name.to_string(),
                    command: cmd.to_string(),
                    found: false,
                },
            }
        })
        .collect();
    info!(
        "cmd::discover_llms found {} LLMs",
        result.iter().filter(|l| l.found).count()
    );
    result
}

#[tauri::command]
pub fn open_external(path: String) -> Result<(), ReasonanceError> {
    info!("cmd::open_external(path={})", path);
    // SEC-06: only allow http:// and https:// schemes to prevent file:// and app:// abuse
    if !path.starts_with("https://") && !path.starts_with("http://") {
        error!(
            "cmd::open_external rejected URL with disallowed scheme: {}",
            path
        );
        return Err(ReasonanceError::Security {
            message: format!("Rejected URL with disallowed scheme: {}", path),
            code: crate::error::SecurityErrorCode::InvalidScheme,
        });
    }
    open::that(&path).map_err(|e| {
        error!("cmd::open_external failed to open {}: {}", path, e);
        ReasonanceError::io(format!("open external '{}'", path), e)
    })
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
    "HOME",
    "USER",
    "SHELL",
    "TERM",
    "XDG_CONFIG_HOME",
];

#[tauri::command]
pub fn get_env_var(name: String) -> Result<Option<String>, ReasonanceError> {
    info!("cmd::get_env_var(name={})", name);
    if !ENV_VAR_ALLOWLIST.contains(&name.as_str()) {
        warn!("cmd::get_env_var rejected non-allowlisted var: {}", name);
        return Err(ReasonanceError::Security {
            message: format!("Environment variable '{}' is not in the allowed list", name),
            code: crate::error::SecurityErrorCode::DisallowedEnvVar,
        });
    }
    // Redact secret values — only confirm presence, never expose actual keys.
    if name.contains("API_KEY") || name.contains("_TOKEN") || name.contains("_SECRET") {
        Ok(std::env::var(&name).ok().map(|_| "***".to_string()))
    } else {
        Ok(std::env::var(&name).ok())
    }
}

#[tauri::command]
pub fn get_system_colors() -> Result<std::collections::HashMap<String, String>, ReasonanceError> {
    info!("cmd::get_system_colors called");
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
                    let parts: Vec<u8> = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
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
        colors.insert(
            "is_dark".into(),
            if luminance < 128.0 { "true" } else { "false" }.into(),
        );
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
