import type { SlashCommand } from '$lib/stores/config';

/**
 * Default slash commands for known LLM CLIs.
 * Keyed by the CLI command name (lowercase).
 */
export const defaultSlashCommands: Record<string, SlashCommand[]> = {
  claude: [
    { command: '/help', description: 'Show available commands' },
    { command: '/clear', description: 'Clear conversation history' },
    { command: '/compact', description: 'Compact conversation context' },
    { command: '/cost', description: 'Show token usage and cost' },
    { command: '/doctor', description: 'Check Claude Code health' },
    { command: '/init', description: 'Initialize project with CLAUDE.md' },
    { command: '/review', description: 'Review code changes' },
    { command: '/bug', description: 'Report a bug' },
    { command: '/seurat', description: 'UI design system, wireframing, accessibility' },
    { command: '/emmet', description: 'Testing, QA, tech debt audit, unit tests' },
    { command: '/heimdall', description: 'Security audit, OWASP, credential detection' },
    { command: '/ghostwriter', description: 'SEO + GEO copywriting optimization' },
    { command: '/baptist', description: 'CRO, A/B testing, funnel analysis' },
    { command: '/orson', description: 'Programmatic video generation' },
    { command: '/scribe', description: 'Office documents and PDF handling' },
    { command: '/forge', description: 'Create, audit, maintain skills' },
  ],
  ollama: [
    { command: '/help', description: 'Show available commands' },
    { command: '/clear', description: 'Clear conversation' },
    { command: '/set', description: 'Set session parameters' },
    { command: '/show', description: 'Show model info' },
    { command: '/load', description: 'Load a model' },
    { command: '/bye', description: 'Exit Ollama' },
  ],
};
