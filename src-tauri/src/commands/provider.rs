use crate::transport::StructuredAgentTransport;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, serde::Serialize)]
struct ConnectionTestStep {
    step: String,
    status: String,
    detail: Option<String>,
}

#[tauri::command]
pub async fn test_provider_connection(
    provider: String,
    transport: State<'_, StructuredAgentTransport>,
    app: AppHandle,
) -> Result<(), String> {
    let (binary, api_key_env, version_cmd) = {
        let registry = transport.registry();
        let reg = registry.lock().unwrap();
        let config = reg.get_config(&provider)
            .ok_or_else(|| format!("Unknown provider: {}", provider))?;
        (
            config.cli.binary.clone(),
            config.cli.api_key_env.clone(),
            config.cli.version_command.clone(),
        )
    };

    // Step 1: Binary check
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "binary".into(),
        status: "checking".into(),
        detail: None,
    });

    let binary_path = which::which(&binary).ok();
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "binary".into(),
        status: if binary_path.is_some() { "ok" } else { "failed" }.into(),
        detail: binary_path.as_ref().map(|p| p.display().to_string()),
    });

    if binary_path.is_none() {
        return Ok(());
    }

    // Step 2: API key check
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "api_key".into(),
        status: "checking".into(),
        detail: None,
    });

    let api_key_set = api_key_env.as_ref()
        .map(|env| std::env::var(env).is_ok())
        .unwrap_or(true);

    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "api_key".into(),
        status: if api_key_set { "ok" } else { "failed" }.into(),
        detail: api_key_env.clone(),
    });

    if !api_key_set {
        return Ok(());
    }

    // Step 3: Connection test (use version_command from TOML if available)
    let _ = app.emit("connection_test_step", ConnectionTestStep {
        step: "connection".into(),
        status: "checking".into(),
        detail: None,
    });

    let start = std::time::Instant::now();
    let output = if !version_cmd.is_empty() {
        tokio::process::Command::new(&version_cmd[0])
            .args(&version_cmd[1..])
            .output()
            .await
    } else {
        tokio::process::Command::new(&binary)
            .args(["--version"])
            .output()
            .await
    };

    match output {
        Ok(o) if o.status.success() => {
            let latency = start.elapsed().as_millis();
            let _ = app.emit("connection_test_step", ConnectionTestStep {
                step: "connection".into(),
                status: "ok".into(),
                detail: Some(format!("{}ms", latency)),
            });
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let _ = app.emit("connection_test_step", ConnectionTestStep {
                step: "connection".into(),
                status: "failed".into(),
                detail: Some(stderr.to_string()),
            });
        }
        Err(e) => {
            let _ = app.emit("connection_test_step", ConnectionTestStep {
                step: "connection".into(),
                status: "failed".into(),
                detail: Some(e.to_string()),
            });
        }
    }

    Ok(())
}

#[tauri::command]
pub fn reload_normalizers(
    transport: State<'_, StructuredAgentTransport>,
) -> Result<(), String> {
    let normalizers_dir = Path::new("normalizers");
    let new_registry = crate::normalizer::NormalizerRegistry::load_from_dir(normalizers_dir)?;
    let registry = transport.registry();
    *registry.lock().unwrap() = new_registry;
    Ok(())
}
