/// End-to-end chat flow integration tests for ALL CLI providers.
///
/// These tests simulate the ENTIRE chat pipeline without running the app:
///   CLI stdout (simulated via `echo`) → StreamReader → NormalizerPipeline
///   → EventBus → HistoryRecorder → event verification
///
/// Covered providers: Claude, Gemini, Kimi, Codex, Qwen
#[cfg(test)]
mod tests {
    use crate::agent_event::{AgentEvent, AgentEventType, EventContent};
    use crate::normalizer::NormalizerRegistry;
    use crate::transport::event_bus::{AgentEventBus, AgentEventSubscriber, HistoryRecorder};
    use crate::transport::stream_reader::spawn_stream_reader;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use tokio::process::Command;

    /// Helper: create a pipeline for a given provider using real TOML configs.
    fn make_pipeline(provider: &str) -> Arc<Mutex<crate::normalizer::pipeline::NormalizerPipeline>> {
        let registry = NormalizerRegistry::load_from_dir(
            std::path::Path::new("normalizers")
        ).unwrap();
        let config = registry.get_config(provider).unwrap();
        let rules = config.to_rules();
        let state_machine: Box<dyn crate::normalizer::state_machines::StateMachine> = match provider {
            "claude" => Box::new(crate::normalizer::state_machines::claude::ClaudeStateMachine::new()),
            "gemini" => Box::new(crate::normalizer::state_machines::gemini::GeminiStateMachine::new()),
            "kimi" => Box::new(crate::normalizer::state_machines::kimi::KimiStateMachine::new()),
            "qwen" => Box::new(crate::normalizer::state_machines::qwen::QwenStateMachine::new()),
            "codex" => Box::new(crate::normalizer::state_machines::codex::CodexStateMachine::new()),
            _ => Box::new(crate::normalizer::state_machines::generic::GenericStateMachine::new()),
        };
        Arc::new(Mutex::new(
            crate::normalizer::pipeline::NormalizerPipeline::new(rules, state_machine, provider.to_string())
        ))
    }

    /// Helper: set up event bus + recorder.
    fn make_bus() -> (Arc<AgentEventBus>, Arc<HistoryRecorder>) {
        let bus = Arc::new(AgentEventBus::new());
        let recorder = Arc::new(HistoryRecorder::new());
        let recorder_clone = recorder.clone();

        struct Wrapper(Arc<HistoryRecorder>);
        impl AgentEventSubscriber for Wrapper {
            fn on_event(&self, session_id: &str, event: &AgentEvent) {
                self.0.on_event(session_id, event);
            }
        }
        bus.subscribe(Box::new(Wrapper(recorder_clone)));
        (bus, recorder)
    }

    /// Helper: spawn a shell that echoes JSON lines to stdout.
    async fn spawn_echo(lines: &[&str]) -> tokio::process::ChildStdout {
        let script = lines.iter()
            .map(|l| format!("printf '%s\\n' '{}'", l.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join("; ");

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&script)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn echo process");

        child.stdout.take().unwrap()
    }

    /// Helper: run a full session and return collected events.
    async fn run_session(provider: &str, lines: &[&str], session_id: &str) -> Vec<AgentEvent> {
        let stdout = spawn_echo(lines).await;
        let pipeline = make_pipeline(provider);
        let (bus, recorder) = make_bus();

        let rx = spawn_stream_reader(stdout, pipeline, bus, session_id.to_string(), None, Arc::new(std::sync::Mutex::new(None)), None);
        let result = rx.await.unwrap();
        assert!(result.error.is_none(), "{}: stream error: {:?}", provider, result.error);

        recorder.get_events(session_id)
    }

    /// Helper: assert events contain at least one Text event with given substring.
    fn assert_has_text(events: &[AgentEvent], substring: &str, provider: &str) {
        let text_events: Vec<_> = events.iter()
            .filter(|e| e.event_type == AgentEventType::Text)
            .collect();
        assert!(!text_events.is_empty(), "{}: no text events found. Got: {:?}",
            provider, events.iter().map(|e| format!("{:?}", e.event_type)).collect::<Vec<_>>());

        let has_content = text_events.iter().any(|e| {
            matches!(&e.content, EventContent::Text { value } if value.contains(substring))
        });
        assert!(has_content, "{}: no text event contains '{}'. Text contents: {:?}",
            provider, substring,
            text_events.iter().filter_map(|e| match &e.content {
                EventContent::Text { value } => Some(value.as_str()),
                _ => None,
            }).collect::<Vec<_>>());
    }

    /// Helper: count events by type.
    fn count_type(events: &[AgentEvent], t: AgentEventType) -> usize {
        events.iter().filter(|e| e.event_type == t).count()
    }

    // ════════════════════════════════════════════════════════════
    //  CLAUDE — type:"assistant" / type:"result" / type:"error"
    // ════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_claude_full_session() {
        let events = run_session("claude", &[
            r#"{"type":"system","subtype":"init","session_id":"s1","model":"claude-sonnet-4-6","cwd":"/tmp"}"#,
            r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","type":"message","role":"assistant","content":[{"type":"thinking","thinking":"Let me think..."}],"usage":{"input_tokens":10,"output_tokens":5}},"session_id":"s1"}"#,
            r#"{"type":"assistant","message":{"id":"msg_1","model":"claude-sonnet-4-6","type":"message","role":"assistant","content":[{"type":"text","text":"Ciao! Come posso aiutarti?"}],"usage":{"input_tokens":10,"output_tokens":15}},"session_id":"s1"}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":3200,"duration_api_ms":3100,"num_turns":1,"result":"Ciao!","stop_reason":"end_turn","session_id":"s1","total_cost_usd":0.042,"usage":{"input_tokens":10,"output_tokens":15,"cache_creation_input_tokens":500,"cache_read_input_tokens":1200}}"#,
        ], "claude-sess").await;

        assert_has_text(&events, "Ciao", "claude");
        assert_eq!(count_type(&events, AgentEventType::Usage), 1, "claude: expected 1 usage event");
        assert_eq!(count_type(&events, AgentEventType::Done), 1, "claude: expected 1 done event");
        assert_eq!(events.last().unwrap().event_type, AgentEventType::Done);

        let usage = events.iter().find(|e| e.event_type == AgentEventType::Usage).unwrap();
        assert_eq!(usage.metadata.input_tokens, Some(10));
        assert_eq!(usage.metadata.output_tokens, Some(15));
        assert_eq!(usage.metadata.cache_creation_tokens, Some(500));
        assert_eq!(usage.metadata.total_cost_usd, Some(0.042));
        assert_eq!(usage.metadata.duration_ms, Some(3200));
    }

    #[tokio::test]
    async fn test_claude_error() {
        let events = run_session("claude", &[
            r#"{"type":"error","message":"Rate limit exceeded","code":"rate_limit"}"#,
        ], "claude-err").await;

        assert_eq!(count_type(&events, AgentEventType::Error), 1);
        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_code, Some("rate_limit".to_string()));
    }

    #[tokio::test]
    async fn test_claude_system_events_ignored() {
        let events = run_session("claude", &[
            r#"{"type":"system","subtype":"init","cwd":"/tmp","session_id":"s1"}"#,
            r#"{"type":"system","subtype":"hook_started","hook_id":"h1"}"#,
            r#"{"type":"system","subtype":"hook_response","hook_id":"h1","output":"ok"}"#,
        ], "claude-sys").await;

        // Only the synthetic Done from stream close
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AgentEventType::Done);
    }

    // ════════════════════════════════════════════════════════════
    //  GEMINI — type:"MESSAGE" / type:"RESULT" / type:"ERROR"
    // ════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_gemini_full_session() {
        let events = run_session("gemini", &[
            r#"{"type":"MESSAGE","content":[{"text":"Ciao! Sono Gemini."}]}"#,
            r#"{"type":"RESULT","usage":{"input_tokens":8,"output_tokens":12,"cached":100,"duration_ms":1500}}"#,
        ], "gemini-sess").await;

        assert_has_text(&events, "Ciao", "gemini");
        assert_eq!(count_type(&events, AgentEventType::Usage), 1, "gemini: expected 1 usage");
        assert!(count_type(&events, AgentEventType::Done) >= 1, "gemini: expected done");

        let usage = events.iter().find(|e| e.event_type == AgentEventType::Usage).unwrap();
        assert_eq!(usage.metadata.input_tokens, Some(8));
        assert_eq!(usage.metadata.output_tokens, Some(12));
        assert_eq!(usage.metadata.cache_read_tokens, Some(100));
        assert_eq!(usage.metadata.duration_ms, Some(1500));
    }

    #[tokio::test]
    async fn test_gemini_tool_use() {
        let events = run_session("gemini", &[
            r#"{"type":"TOOL_USE","name":"read_file","id":"tu_1","args":"{\"path\":\"/tmp/test.txt\"}"}"#,
            r#"{"type":"TOOL_RESULT","tool_use_id":"tu_1","result":"file contents here"}"#,
            r#"{"type":"MESSAGE","content":[{"text":"I read the file for you."}]}"#,
            r#"{"type":"RESULT","usage":{"input_tokens":20,"output_tokens":10}}"#,
        ], "gemini-tool").await;

        assert!(count_type(&events, AgentEventType::ToolUse) >= 1, "gemini: expected tool_use");
        assert!(count_type(&events, AgentEventType::ToolResult) >= 1, "gemini: expected tool_result");
        assert_has_text(&events, "read the file", "gemini");
    }

    #[tokio::test]
    async fn test_gemini_error_resource_exhausted() {
        let events = run_session("gemini", &[
            r#"{"type":"ERROR","code":"RESOURCE_EXHAUSTED","message":"Quota exceeded"}"#,
        ], "gemini-err").await;

        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_code, Some("RESOURCE_EXHAUSTED".to_string()));
        // resource_exhausted is configured as recoverable severity
        assert_eq!(err.metadata.error_severity, Some(crate::agent_event::ErrorSeverity::Recoverable));
    }

    #[tokio::test]
    async fn test_gemini_error_generic_is_fatal() {
        let events = run_session("gemini", &[
            r#"{"type":"ERROR","code":"UNKNOWN","message":"Something broke"}"#,
        ], "gemini-fatal").await;

        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_severity, Some(crate::agent_event::ErrorSeverity::Fatal));
    }

    // ════════════════════════════════════════════════════════════
    //  KIMI — content_block_delta / message_delta / message_stop
    // ════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_kimi_full_session() {
        let events = run_session("kimi", &[
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Ciao da Kimi!"}}"#,
            r#"{"type":"message_delta","usage":{"input_tokens":6,"output_tokens":8}}"#,
            r#"{"type":"message_stop"}"#,
        ], "kimi-sess").await;

        assert_has_text(&events, "Ciao da Kimi", "kimi");
        assert_eq!(count_type(&events, AgentEventType::Usage), 1, "kimi: expected 1 usage");
        // Both explicit done (message_stop) and synthetic done (stream close)
        assert!(count_type(&events, AgentEventType::Done) >= 1, "kimi: expected done");

        let usage = events.iter().find(|e| e.event_type == AgentEventType::Usage).unwrap();
        assert_eq!(usage.metadata.input_tokens, Some(6));
        assert_eq!(usage.metadata.output_tokens, Some(8));
    }

    #[tokio::test]
    async fn test_kimi_thinking() {
        let events = run_session("kimi", &[
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"Let me reason about this..."}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"The answer is 4."}}"#,
            r#"{"type":"message_stop"}"#,
        ], "kimi-think").await;

        assert!(count_type(&events, AgentEventType::Thinking) >= 1, "kimi: expected thinking event");
        assert_has_text(&events, "answer is 4", "kimi");
    }

    #[tokio::test]
    async fn test_kimi_tool_use() {
        let events = run_session("kimi", &[
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","name":"bash","id":"tu_1","input":""}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"cmd\":\"ls\"}"}}"#,
            r#"{"type":"content_block_stop"}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Done!"}}"#,
            r#"{"type":"message_stop"}"#,
        ], "kimi-tool").await;

        assert!(count_type(&events, AgentEventType::ToolUse) >= 1, "kimi: expected tool_use");
        let tool = events.iter().find(|e| e.event_type == AgentEventType::ToolUse).unwrap();
        assert_eq!(tool.metadata.tool_name, Some("bash".to_string()));
    }

    #[tokio::test]
    async fn test_kimi_error_overloaded() {
        let events = run_session("kimi", &[
            r#"{"type":"error","error":{"type":"overloaded","message":"Server busy"}}"#,
        ], "kimi-err").await;

        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_code, Some("overloaded".to_string()));
        assert_eq!(err.metadata.error_severity, Some(crate::agent_event::ErrorSeverity::Recoverable));
    }

    #[tokio::test]
    async fn test_kimi_context_metrics() {
        let events = run_session("kimi", &[
            r#"{"type":"message_delta","context_usage":0.75,"context_tokens":96000,"max_context_tokens":128000}"#,
            r#"{"type":"message_stop"}"#,
        ], "kimi-ctx").await;

        let metrics: Vec<_> = events.iter().filter(|e| e.event_type == AgentEventType::Metrics).collect();
        assert_eq!(metrics.len(), 1, "kimi: expected 1 metrics event");
        assert_eq!(metrics[0].metadata.context_usage, Some(0.75));
        assert_eq!(metrics[0].metadata.context_tokens, Some(96000));
        assert_eq!(metrics[0].metadata.max_context_tokens, Some(128000));
    }

    // ════════════════════════════════════════════════════════════
    //  CODEX — JSON-RPC style: method + params
    // ════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_codex_full_session() {
        let events = run_session("codex", &[
            r#"{"method":"AgentMessageDeltaNotification","params":{"delta":"Ciao da Codex!"}}"#,
            r#"{"method":"ThreadTokenUsageUpdatedNotification","params":{"usage":{"input_tokens":12,"output_tokens":20,"cachedInputTokens":50}}}"#,
            r#"{"method":"TurnCompletedNotification","params":{}}"#,
        ], "codex-sess").await;

        assert_has_text(&events, "Ciao da Codex", "codex");
        assert_eq!(count_type(&events, AgentEventType::Usage), 1, "codex: expected 1 usage");
        // Explicit done (TurnCompleted) + synthetic done (stream close)
        assert!(count_type(&events, AgentEventType::Done) >= 1, "codex: expected done");

        let usage = events.iter().find(|e| e.event_type == AgentEventType::Usage).unwrap();
        assert_eq!(usage.metadata.input_tokens, Some(12));
        assert_eq!(usage.metadata.output_tokens, Some(20));
        assert_eq!(usage.metadata.cache_read_tokens, Some(50));
    }

    #[tokio::test]
    async fn test_codex_reasoning() {
        let events = run_session("codex", &[
            r#"{"method":"ItemCompletedNotification","params":{"item":{"type":"reasoning","content":"I need to think step by step..."}}}"#,
            r#"{"method":"AgentMessageDeltaNotification","params":{"delta":"The answer is 42."}}"#,
            r#"{"method":"TurnCompletedNotification","params":{}}"#,
        ], "codex-reason").await;

        assert!(count_type(&events, AgentEventType::Thinking) >= 1, "codex: expected thinking");
        assert_has_text(&events, "answer is 42", "codex");
    }

    #[tokio::test]
    async fn test_codex_command_execution() {
        let events = run_session("codex", &[
            r#"{"method":"ItemCompletedNotification","params":{"item":{"type":"commandExecution","command":"ls -la","output":"total 42\ndrwxr-xr-x ...","id":"cmd_1"}}}"#,
            r#"{"method":"AgentMessageDeltaNotification","params":{"delta":"I listed the files."}}"#,
            r#"{"method":"TurnCompletedNotification","params":{}}"#,
        ], "codex-cmd").await;

        let tools: Vec<_> = events.iter().filter(|e| e.event_type == AgentEventType::ToolUse).collect();
        assert!(!tools.is_empty(), "codex: expected tool_use for command execution");
        assert_eq!(tools[0].metadata.tool_name, Some("ls -la".to_string()));
    }

    #[tokio::test]
    async fn test_codex_error_rate_limit() {
        let events = run_session("codex", &[
            r#"{"method":"ErrorNotification","params":{"message":"Too many requests","code":"rate_limit"}}"#,
        ], "codex-err").await;

        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_code, Some("rate_limit".to_string()));
        assert_eq!(err.metadata.error_severity, Some(crate::agent_event::ErrorSeverity::Recoverable));
    }

    #[tokio::test]
    async fn test_codex_error_generic_is_fatal() {
        let events = run_session("codex", &[
            r#"{"method":"ErrorNotification","params":{"message":"Unknown error","code":"unknown"}}"#,
        ], "codex-fatal").await;

        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_severity, Some(crate::agent_event::ErrorSeverity::Fatal));
    }

    // ════════════════════════════════════════════════════════════
    //  QWEN — hybrid: content_block_delta + assistant/result
    // ════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_qwen_streaming_session() {
        let events = run_session("qwen", &[
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Ciao da Qwen!"}}"#,
            r#"{"type":"result","usage":{"input_tokens":7,"output_tokens":9},"duration_ms":2000,"duration_api_ms":1900,"num_turns":1}"#,
        ], "qwen-stream").await;

        assert_has_text(&events, "Ciao da Qwen", "qwen");
        assert_eq!(count_type(&events, AgentEventType::Usage), 1, "qwen: expected 1 usage");

        let usage = events.iter().find(|e| e.event_type == AgentEventType::Usage).unwrap();
        assert_eq!(usage.metadata.input_tokens, Some(7));
        assert_eq!(usage.metadata.output_tokens, Some(9));
        assert_eq!(usage.metadata.duration_ms, Some(2000));
        assert_eq!(usage.metadata.num_turns, Some(1));
    }

    #[tokio::test]
    async fn test_qwen_assistant_fallback() {
        // Qwen also supports the assistant/result format (fallback)
        let events = run_session("qwen", &[
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Fallback response"}]}}"#,
            r#"{"type":"result","usage":{"input_tokens":5,"output_tokens":3}}"#,
        ], "qwen-fallback").await;

        assert_has_text(&events, "Fallback response", "qwen");
        assert_eq!(count_type(&events, AgentEventType::Usage), 1, "qwen: expected usage from result");
    }

    #[tokio::test]
    async fn test_qwen_tool_use() {
        let events = run_session("qwen", &[
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","name":"write_file","id":"tu_1","input":""}}"#,
            r#"{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/tmp/test\"}"}}"#,
            r#"{"type":"content_block_stop"}"#,
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"File written."}}"#,
            r#"{"type":"result","usage":{"input_tokens":15,"output_tokens":8}}"#,
        ], "qwen-tool").await;

        assert!(count_type(&events, AgentEventType::ToolUse) >= 1, "qwen: expected tool_use");
        assert_has_text(&events, "File written", "qwen");
    }

    #[tokio::test]
    async fn test_qwen_error_overloaded() {
        let events = run_session("qwen", &[
            r#"{"type":"error","error":{"type":"overloaded","message":"Server overloaded"}}"#,
        ], "qwen-err").await;

        let err = events.iter().find(|e| e.event_type == AgentEventType::Error).unwrap();
        assert_eq!(err.metadata.error_code, Some("overloaded".to_string()));
        assert_eq!(err.metadata.error_severity, Some(crate::agent_event::ErrorSeverity::Recoverable));
    }

    // ════════════════════════════════════════════════════════════
    //  CROSS-PROVIDER — common behavior
    // ════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_all_providers_set_provider_metadata() {
        // Codex accumulates text deltas and only flushes on a non-delta event
        // (like TurnCompleted), so it needs two lines.
        let provider_lines: Vec<(&str, Vec<&str>)> = vec![
            ("claude", vec![r#"{"type":"assistant","message":{"id":"m1","model":"x","content":[{"type":"text","text":"hi"}]}}"#]),
            ("gemini", vec![r#"{"type":"MESSAGE","content":[{"text":"hi"}]}"#]),
            ("kimi", vec![r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}"#]),
            ("codex", vec![
                r#"{"method":"AgentMessageDeltaNotification","params":{"delta":"hi"}}"#,
                r#"{"method":"TurnCompletedNotification","params":{}}"#,
            ]),
            ("qwen", vec![r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}"#]),
        ];

        for (provider, lines) in &provider_lines {
            let events = run_session(provider, lines, &format!("{}-meta", provider)).await;
            let text_events: Vec<_> = events.iter()
                .filter(|e| e.event_type == AgentEventType::Text)
                .collect();

            assert!(!text_events.is_empty(), "{}: should produce text event", provider);
            assert_eq!(text_events[0].metadata.provider, *provider,
                "{}: provider metadata should be '{}'", provider, provider);
        }
    }

    #[tokio::test]
    async fn test_all_providers_emit_done_on_stream_close() {
        for provider in &["claude", "gemini", "kimi", "codex", "qwen"] {
            // Empty stream — only synthetic done
            let events = run_session(provider, &[
                r#"{"type":"ping"}"#, // no rule matches this for any provider
            ], &format!("{}-done", provider)).await;

            assert!(count_type(&events, AgentEventType::Done) >= 1,
                "{}: should emit synthetic Done on stream close", provider);
        }
    }

    #[tokio::test]
    async fn test_all_providers_handle_invalid_json() {
        for provider in &["claude", "gemini", "kimi", "codex", "qwen"] {
            let events = run_session(provider, &[
                "not json",
                "{broken",
            ], &format!("{}-invalid", provider)).await;

            // Should not crash — only synthetic done
            assert!(count_type(&events, AgentEventType::Done) >= 1,
                "{}: should survive invalid JSON", provider);
        }
    }

    #[tokio::test]
    async fn test_session_id_isolation_across_providers() {
        let (bus, recorder) = make_bus();

        // Run two providers on the SAME bus with different session IDs
        let stdout1 = spawn_echo(&[
            r#"{"type":"assistant","message":{"id":"m1","model":"x","content":[{"type":"text","text":"from claude"}]}}"#,
        ]).await;
        let stdout2 = spawn_echo(&[
            r#"{"type":"MESSAGE","content":[{"text":"from gemini"}]}"#,
        ]).await;

        let pipeline1 = make_pipeline("claude");
        let pipeline2 = make_pipeline("gemini");

        let rx1 = spawn_stream_reader(stdout1, pipeline1, bus.clone(), "sid-claude".to_string(), None, Arc::new(std::sync::Mutex::new(None)), None);
        let rx2 = spawn_stream_reader(stdout2, pipeline2, bus.clone(), "sid-gemini".to_string(), None, Arc::new(std::sync::Mutex::new(None)), None);

        rx1.await.unwrap();
        rx2.await.unwrap();

        let claude_events = recorder.get_events("sid-claude");
        let gemini_events = recorder.get_events("sid-gemini");

        assert_has_text(&claude_events, "from claude", "claude");
        assert_has_text(&gemini_events, "from gemini", "gemini");

        // No cross-contamination
        let claude_text: String = claude_events.iter().filter_map(|e| match &e.content {
            EventContent::Text { value } => Some(value.clone()), _ => None
        }).collect();
        assert!(!claude_text.contains("gemini"), "claude session should not contain gemini text");
    }
}
