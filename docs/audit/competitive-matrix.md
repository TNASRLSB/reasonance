# Competitive Matrix: Reasonance vs Industry

**Date:** 2026-03-22
**Status:** Best-effort — requires human validation

---

## Feature Comparison

| Feature | Reasonance | VS Code | Cursor | Zed | Windsurf |
|---------|-----------|---------|--------|-----|----------|
| Screen reader support | ⚠️ Partial (ARIA present, no dedicated mode) | ✅ Full (NVDA, JAWS, VoiceOver; dedicated SR mode) | ❌ Major gaps (AI suggestions unreadable, generate dialog inaccessible) | ❌ Menus only; editor content invisible to VoiceOver | ❌ No documented support |
| Keyboard-only navigation | ⚠️ Partial (tabindex in some components, gaps remain) | ✅ Full (all UI navigable) | ⚠️ Partial (dialog buttons not tab-reachable) | ⚠️ Partial (standard editing, limited custom UI) | ⚠️ Inherited from VS Code fork |
| High contrast mode | ❌ Not implemented | ✅ Built-in High Contrast theme + High Contrast Light | ❌ Not documented | ❌ Theme-based only; contrast issues reported | ❌ Not documented |
| Reduced motion | ⚠️ Partial (`prefers-reduced-motion` in skeleton only) | ✅ Respected across UI | ❌ Not documented | ❌ Not documented | ❌ Not documented |
| RTL support | ⚠️ Arabic locale exists; no CSS logical properties | ✅ Full RTL support | ❌ Not documented | ❌ Not documented | ❌ Not documented |
| WCAG 2.1 AA compliance | ❌ Not certified; partial adherence | ⚠️ Follows WCAG 2.0 guidelines internally | ❌ Community reports significant gaps | ❌ Acknowledged as long-term project | ❌ Not documented |
| Multi-AI support | ✅ Native multi-provider (Claude, Gemini, Kimi, Qwen, Codex) | ⚠️ Via extensions (Copilot built-in) | ✅ Multiple models | ✅ Multiple models | ✅ Multiple models |
| Native performance | ✅ Tauri 2 + Rust backend | ❌ Electron | ❌ Electron fork | ✅ Native (GPUI) | ❌ Electron fork |
| i18n (languages) | 9 locales | 50+ locales | ~10 locales | ~5 locales | ~10 locales |
| Built-in analytics | ✅ Cost/usage tracking | ❌ Telemetry only | ❌ Basic usage stats | ❌ None | ❌ None |
| Workflow orchestration | ✅ Swarm/workflow builder | ⚠️ Via extensions | ❌ None | ❌ None | ❌ Cascade (linear only) |

---

## Accessibility Deep Dive

### VS Code
- **Gold standard** for editor accessibility. Dedicated screen reader mode auto-detects NVDA/JAWS/VoiceOver.
- Every new UI component must pass accessibility review per internal WCAG 2.0 guidelines.
- Built-in high contrast themes (dark and light), zoom support, keyboard-only navigation for all features.
- Screen reader announces suggestions, navigation, and editor content.
- Extensive documentation at [code.visualstudio.com/docs/configure/accessibility](https://code.visualstudio.com/docs/configure/accessibility).

### Cursor
- Fork of VS Code, but AI-specific features have **significant accessibility gaps**.
- Screen reader users report: AI suggestions not readable, `Ctrl+K` generate dialog inaccessible, tab key doesn't reach dialog buttons.
- Community forum has multiple open accessibility requests (2024-2025) with no resolution.
- Inherits VS Code's base accessibility but breaks it in AI-overlay features.

### Zed
- Native GPU-rendered editor — custom rendering pipeline means **no OS accessibility hooks** for editor content.
- Menus are accessible, but VoiceOver sees "empty window" for editor area.
- Team acknowledges a11y as "long-term project, far beyond 1.0."
- Color contrast issues in default themes (highlights/selections use transparency blending).
- GitHub Discussion [#6576](https://github.com/zed-industries/zed/discussions/6576) tracks the effort.

### Windsurf
- VS Code fork by Codeium. **No documented accessibility features** beyond inherited VS Code base.
- Marketing focuses on "accessible" meaning "easy to use" — not disability accessibility.
- Command palette and standard VS Code keyboard shortcuts work.
- No evidence of screen reader testing, high contrast support, or WCAG evaluation.

---

## Where Reasonance Leads

1. **Multi-provider AI native** — Only IDE with built-in support for 5+ AI providers simultaneously, with a normalizer layer abstracting protocol differences.
2. **Native performance** — Tauri 2 + Rust avoids Electron's memory overhead. Shared advantage with Zed only.
3. **Built-in analytics** — Cost tracking and usage analytics with no extensions needed.
4. **Workflow orchestration** — Visual swarm/workflow builder for multi-agent pipelines.
5. **i18n breadth for an AI IDE** — 9 locales vs Cursor/Windsurf's ~10 and Zed's ~5 (though VS Code leads at 50+).

---

## Where Reasonance Trails

1. **Accessibility maturity** — VS Code has years of dedicated accessibility work. Reasonance has ARIA attributes in many components but lacks a dedicated screen reader mode, high contrast theme, and systematic WCAG compliance.
2. **i18n depth** — 9 locales exist but RTL (Arabic) lacks CSS logical properties, and locale completeness may vary.
3. **Reduced motion** — Only skeleton component respects `prefers-reduced-motion`; other animations may not.
4. **Extension ecosystem** — VS Code's marketplace is unmatched. Reasonance has no extension system.
5. **Community size and testing** — VS Code and Cursor have massive user bases providing feedback; Reasonance is early-stage.

---

## Where Nobody Leads (Industry Gaps)

1. **AI feature accessibility** — No AI IDE (Cursor, Windsurf, Zed) has made AI-specific features fully accessible to screen reader users. This is an open opportunity.
2. **WCAG AA certification for AI IDEs** — No AI-native IDE has achieved formal WCAG AA compliance. VS Code comes closest but isn't AI-native.
3. **RTL-first AI coding** — No IDE provides a first-class RTL coding experience. Arabic/Hebrew developers are underserved across the board.
4. **Cognitive accessibility** — No IDE specifically addresses cognitive load, ADHD-friendly modes, or neurodiversity accommodations.
5. **Forced-colors / Windows High Contrast** — Even VS Code's support is incomplete. No competitor handles `forced-colors: active` comprehensively.

---

## Strategic Opportunity for Reasonance

Reasonance can differentiate by being the **first AI-native IDE with genuine accessibility**. The competition is weak here:
- Cursor/Windsurf broke VS Code's accessibility with their AI overlays
- Zed chose native rendering, sacrificing OS a11y hooks
- No one has solved "accessible AI features"

If Reasonance achieves WCAG AA compliance with accessible AI interactions, it occupies a unique market position no competitor currently claims.
