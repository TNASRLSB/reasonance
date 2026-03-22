import type { CliMode } from './agent-event';

export interface NegotiatedCapabilities {
  provider: string;
  cli_version: string;
  cli_mode: CliMode;
  features: Record<string, FeatureSupport>;
  negotiated_at: number;
}

export type FeatureSupport =
  | { level: 'full' }
  | { level: 'partial'; limitations: string[]; workaround?: Workaround }
  | { level: 'unsupported'; alternative?: Workaround };

export interface Workaround {
  description: string;
  method: WorkaroundMethod;
}

export type WorkaroundMethod =
  | 'inline_in_prompt'
  | 'simulate_from_batch'
  | { fallback_flag: string }
  | 'skip_silently';

export interface CliVersionInfo {
  provider: string;
  current_version: string | null;
  last_checked: number | null;
  auto_update: boolean;
  version_command: string[];
  update_command: string[];
}

export interface VersionEntry {
  id: string;
  provider: string;
  timestamp: number;
  checksum: string;
}

export interface HealthReport {
  provider: string;
  status: HealthStatus;
  results: TestCaseResult[];
  capabilities_confirmed: string[];
  capabilities_missing: string[];
  capabilities_broken: string[];
  tested_at: number;
  cli_version: string;
}

export type HealthStatus =
  | { type: 'healthy' }
  | { type: 'degraded'; failing_tests: string[] }
  | { type: 'broken'; error: string };

export interface TestCaseResult {
  name: string;
  passed: boolean;
  failure_reason: string | null;
}
