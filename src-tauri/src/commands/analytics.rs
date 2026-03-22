use crate::analytics::{ProviderAnalytics, ModelAnalytics, DailyStats, SessionMetrics, TimeRange};
use crate::analytics::collector::AnalyticsCollector;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn analytics_provider(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<ProviderAnalytics, String> {
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.get_provider_analytics(&provider, range))
}

#[tauri::command]
pub fn analytics_compare(
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<ProviderAnalytics>, String> {
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.compare_providers(range))
}

#[tauri::command]
pub fn analytics_model_breakdown(
    provider: String,
    from: Option<u64>,
    to: Option<u64>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<ModelAnalytics>, String> {
    let range = if from.is_some() || to.is_some() {
        Some(TimeRange { from, to })
    } else {
        None
    };
    Ok(collector.get_model_breakdown(&provider, range))
}

#[tauri::command]
pub fn analytics_session(
    session_id: String,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Option<SessionMetrics>, String> {
    Ok(collector.get_session_metrics(&session_id))
}

#[tauri::command]
pub fn analytics_daily(
    provider: Option<String>,
    days: Option<u32>,
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<DailyStats>, String> {
    Ok(collector.get_daily_stats(provider.as_deref(), days.unwrap_or(30)))
}

#[tauri::command]
pub fn analytics_active(
    collector: State<'_, Arc<AnalyticsCollector>>,
) -> Result<Vec<SessionMetrics>, String> {
    Ok(collector.get_active_sessions())
}
