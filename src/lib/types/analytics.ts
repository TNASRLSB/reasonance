// src/lib/types/analytics.ts

// === Backend mirrors (match Rust serde output from src-tauri/src/analytics/mod.rs) ===

export interface ErrorRecord {
  timestamp: number;
  code: string;
  severity: string; // ErrorSeverity enum serialized as string
  recovered: boolean;
}

export interface SessionMetrics {
  session_id: string;
  provider: string;
  model: string;
  started_at: number;
  ended_at: number | null;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  duration_ms: number | null;
  duration_api_ms: number | null;
  num_turns: number;
  tools_used: Record<string, number>;
  stop_reason: string | null;
  peak_context_usage: number | null;
  max_context_tokens: number | null;
  total_cost_usd: number | null;
  errors: ErrorRecord[];
}

export interface ProviderAnalytics {
  provider: string;
  total_sessions: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_creation_tokens: number;
  total_cache_read_tokens: number;
  cache_hit_rate: number;
  total_errors: number;
  recovered_errors: number;
  error_rate: number;
  avg_duration_ms: number;
  avg_tokens_per_second: number;
  most_used_model: string;
  total_tool_invocations: number;
  total_cost_usd: number | null;
}

export interface ModelAnalytics {
  model: string;
  provider: string;
  session_count: number;
  avg_input_tokens: number;
  avg_output_tokens: number;
  avg_duration_ms: number;
  avg_tokens_per_second: number;
  error_rate: number;
}

export interface DailyStats {
  date: string;
  provider: string | null;
  sessions: number;
  input_tokens: number;
  output_tokens: number;
  errors: number;
  avg_duration_ms: number;
  total_cost_usd?: number;
}

export interface TimeRange {
  from: number | null;
  to: number | null;
}

// === Frontend-only types ===

export interface LiveSessionMetrics {
  session_id: string;
  provider: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
  duration_ms: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  context_percent: number | null;
  num_turns: number;
  errors: number;
  errors_recovered: number;
  is_streaming: boolean;
  cost_velocity_usd_per_min: number;
  cost_projection_usd: number | null;
  vs_avg_ratio: number | null;
}

export interface AnalyticsBudget {
  daily_limit_usd: number | null;
  weekly_limit_usd: number | null;
  notify_at_percent: number;
}

export interface BudgetAlert {
  type: 'approaching' | 'exceeded';
  period: 'daily' | 'weekly';
  current_usd: number;
  limit_usd: number;
}

export interface ConnectionTestStep {
  step: 'binary' | 'api_key' | 'connection';
  status: 'checking' | 'ok' | 'failed';
  detail: string | null;
}

export interface DayHealth {
  date: string;
  status: 'ok' | 'degraded' | 'down' | 'unused';
  errors: number;
  recovered: number;
}

export interface StoreState<T> {
  data: T | null;
  status: 'idle' | 'loading' | 'ready' | 'error';
  error: string | null;
}

export interface AnalyticsDashboardState {
  open: boolean;
  focus: {
    provider?: string;
    sessionId?: string;
    section?: 'kpi' | 'insights' | 'providers' | 'trend';
  } | null;
}

export interface BarValue {
  key: string;
  value: number;
  widthPercent: number;
}

export interface ProviderVisual {
  color: string;
  pattern: string;
  patternLabel: string;
  contrastColor: string;
  backgroundSize?: string;
}

export interface ModelInfo {
  id: string;
  provider: string;
  name: string;
  cost_per_1m_input: number;
  cost_per_1m_output: number;
  context_window: number;
}
