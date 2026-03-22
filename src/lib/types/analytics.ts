// src/lib/types/analytics.ts

// === Backend mirrors (match Rust serde output) ===

export interface SessionMetrics {
  session_id: string;
  provider: string;
  model: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
  duration_ms: number;
  duration_api_ms: number;
  num_turns: number;
  tool_calls: number;
  errors_total: number;
  errors_recovered: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  stop_reason: string | null;
  started_at: number;
  completed_at: number | null;
}

export interface ProviderAnalytics {
  provider: string;
  total_sessions: number;
  total_cost_usd: number;
  total_input_tokens: number;
  total_output_tokens: number;
  avg_duration_ms: number;
  avg_tokens_per_session: number;
  error_rate: number;
  cache_hit_rate: number;
}

export interface ModelAnalytics {
  model: string;
  provider: string;
  total_sessions: number;
  total_cost_usd: number;
  total_tokens: number;
  avg_duration_ms: number;
}

export interface DailyStats {
  date: string;
  sessions: number;
  total_cost_usd: number;
  total_tokens: number;
  providers_used: string[];
}

export interface TimeRange {
  start_epoch_ms: number;
  end_epoch_ms: number;
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
