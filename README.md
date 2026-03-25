<p align="center">
  <img src="static/favicon.png" alt="Reasonance" width="120" />
</p>

<h1 align="center">Reasonance</h1>

<p align="center">
  <strong>The IDE that thinks with you.</strong><br/>
  AI-native. Accessible. Blazing fast.
</p>

<p align="center">
  <a href="https://github.com/TNASRLSB/reasonance/releases/latest"><img src="https://img.shields.io/github/v/release/TNASRLSB/reasonance?style=flat-square" alt="Latest Release" /></a>
  <a href="https://github.com/TNASRLSB/reasonance/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/TNASRLSB/reasonance/release.yml?branch=main&style=flat-square" alt="Build Status" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/TNASRLSB/reasonance?style=flat-square" alt="License" /></a>
</p>

<p align="center">
  <a href="#features">Features</a> &middot;
  <a href="#why-reasonance">Why Reasonance</a> &middot;
  <a href="#accessibility">Accessibility</a> &middot;
  <a href="#download">Download</a> &middot;
  <a href="#build-from-source">Build from Source</a> &middot;
  <a href="#license">License</a>
</p>

---

## The Problem

Today's development tools were built for a world without AI. They bolt on chat panels and call it innovation. Meanwhile, you're still copy-pasting between terminal and editor, losing context every time you switch tools, and watching your IDE eat 2 GB of RAM doing nothing.

**Reasonance starts from a different premise:** AI isn't an add-on. It's how you work.

---

## Why Reasonance

**Intent over keystrokes.** You describe what you want. Reasonance and your AI of choice figure out how to get there. No more fighting the tool to express your idea.

**Built for every human.** Not an afterthought — accessibility is the architecture. Screen readers, keyboard navigation, high contrast, reduced motion, enhanced readability. Same power, every modality. WCAG 2.1 AA conformant ([audit status](docs/compliance/wcag-matrix.md) · [VPAT](docs/compliance/vpat-2.4.md) · [EN 301 549](docs/compliance/en-301-549.md)).

**Secure by design.** API keys never touch the browser. All LLM calls are proxied through the native Rust backend. Markdown output is sanitized against XSS. Per-model permission levels and workspace trust keep you in control.

**Native speed, tiny footprint.** Built on Tauri + Svelte. Sub-100 MB memory. Starts in under a second. No Electron. No bloat.

---

## Features

### Multi-AI Terminal
Run Claude, Gemini, GPT, or local models side by side. Each in its own tab, each with full context. Switch between them like browser tabs — compare approaches, pick what works. Auto-detection of installed LLM CLIs with extended PATH search.

### Real-Time Context Awareness
See exactly how much context your AI has left. A live progress bar shows token usage so you never hit the wall mid-thought.

### Brutalist Code Editor
CodeMirror 6 with a distinctive brutalist theme. Fast, precise, no distractions. Syntax highlighting, intelligent completions, and a look that means business.

### Smart File Tree
Your project at a glance. Navigate, open, explore — all keyboard-accessible, all instant. Respects `.gitignore`, shows git status, and supports full keyboard navigation with arrow keys.

### Theme Editor
Full visual theme customization with live preview. Compose themes from modifiers (high contrast, enhanced readability, reduced motion). WCAG contrast ratio badges show AA/AAA compliance in real time. Edit as JSON for advanced control. System color integration reads your desktop palette.

### AI Context Menu
Select code, right-click, and ask any connected AI to explain, refactor, or extend it. No copy-paste. No context switching.

### Built-In Diff View
Side-by-side comparison with accept/reject controls. Syntax-highlighted, with accessible `aria-expanded` states and non-color diff indicators.

### Command Palette
Quick access to every action via `Ctrl+P` / `Cmd+P`. Search commands, open files, switch settings — all without leaving the keyboard.

### Find in Files
Project-wide search with regex support. Results in context, respects `.gitignore`.

### Per-Model Permission System
Fine-grained control over what each AI can do. Three levels per model: **YOLO** (auto-approve), **Ask** (interactive approve/deny with "Approve & Remember"), **Locked** (deny all). Persistent allowed-tools configuration per provider.

### Session Management
Full session persistence with fork, restore, and rename. Replay past conversations, re-run tool approvals, and pick up where you left off. Multiple concurrent sessions.

### Analytics Dashboard
Live token count, cost tracking, cache efficiency, and token velocity. Per-provider comparison, per-model breakdowns, daily trend charts, and health status monitoring. Budget alerts with configurable daily/weekly spending limits.

### Provider Health Monitoring
Real-time health checks for all configured LLM providers — binary verification, API key validation, and connection testing. Status history (ok/degraded/down) with automatic tracking.

### Hive Canvas
Visual dataflow editor for AI orchestration. Drag-and-drop Agent, Logic, and Resource nodes to build workflows visually. Powered by xyflow/SvelteFlow. Dual mode (visual + code), step-through debugging, node inspector with JSON toggle. Sandboxed Rhai logic evaluation and resource locking.

### Inline Updater
Automatic update detection with in-app download and install for both the app and LLM CLIs. Postpone updates when you're in the flow.

### Internationalization
Full i18n support with 9 languages: English, Arabic, German, Spanish, French, Hindi, Italian, Portuguese, Chinese. RTL layout support. All UI strings including ARIA labels are translated.

### Workspace Trust
Per-workspace trust levels (Untrusted, Limited, Full). Control which workspaces can execute code and access system resources. Revokable at any time.

### In-App Help
Searchable documentation panel with locale-aware content. Full-text search with highlighted results. Falls back to English when a translation isn't available.

### Image Drop
Paste or drag images directly into conversations. No file picker required.

### Welcome Screen
Guided onboarding for first-time setup — provider configuration, workspace selection, and feature overview.

---

## Accessibility

Accessibility is not a feature — it is the architecture. Reasonance targets **WCAG 2.1 Level AA** conformance and publishes formal compliance documentation:

- **[WCAG 2.1 Audit Matrix](docs/compliance/wcag-matrix.md)** — component-by-component, criterion-by-criterion
- **[VPAT 2.4 (Section 508)](docs/compliance/vpat-2.4.md)** — US federal accessibility standard
- **[EN 301 549](docs/compliance/en-301-549.md)** — European Accessibility Act compliance

### What this means in practice

- **Atkinson Hyperlegible** font family — designed for maximum character distinguishability
- **Skip links** to file tree, editor, and terminal for screen reader users
- **Focus management** — stack-based focus restoration, roving tabindex in tabs and toolbars, focus traps in all dialogs
- **Screen reader announcer** — dual `aria-live` regions (polite + assertive) with throttling
- **Keyboard navigation** everywhere — file tree (arrows, Enter/Space), menus (arrow keys, Home/End), panels, dialogs (Escape to close)
- **High contrast mode** — composable theme modifier exceeding AAA contrast ratios
- **Enhanced readability mode** — larger text, increased letter-spacing, adjusted line-height
- **Reduced motion** — respects `prefers-reduced-motion: reduce` across all animations (8+ implementations)
- **Non-color indicators** — diff markers, status labels, and ARIA states supplement all color-coded information
- **Minimum target size** — 32px for primary controls, 24px minimum for all interactive elements
- **Full ARIA coverage** — 476+ ARIA attributes across 53 files: roles, labels, live regions, expanded states

---

## Download

### Pre-built binaries

Download the latest release for your platform:

**[GitHub Releases](https://github.com/TNASRLSB/reasonance/releases/latest)** — Linux (.deb, .AppImage, .rpm), macOS (.dmg), Windows (.msi)

### Arch Linux (AUR)

```bash
yay -S reasonance
```

---

## Build from Source

### Requirements
- Linux, macOS, or Windows
- Rust toolchain
- Node.js 20+

```bash
git clone https://github.com/TNASRLSB/reasonance.git
cd reasonance
npm install
npm run tauri dev
```

---

## Architecture

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop shell | **Tauri 2** | Native performance, tiny binary, system webview |
| UI framework | **Svelte 5** | Compile-time reactivity, zero runtime overhead |
| Code editor | **CodeMirror 6** | Modular, extensible, accessible |
| Terminal | **xterm.js + PTY** | Real terminal emulation, not a simulation |
| Workflow graph | **xyflow / SvelteFlow** | Interactive node editor for Hive Canvas |
| Typography | **Atkinson Hyperlegible** | Designed for readability, not aesthetics alone |

---

## Roadmap

- [x] Visual dataflow editor for AI orchestration (Hive Canvas)
- [x] Per-model permission system with replay
- [x] WCAG 2.1 AA conformance
- [x] 9-language internationalization with RTL
- [x] Analytics dashboard with budget alerts
- [x] Theme editor with WCAG contrast badges
- [ ] Multi-modal input (voice, switch, eye tracking)
- [ ] Persistent decision log across sessions
- [ ] Plugin ecosystem
- [ ] Collaborative multi-user sessions

---

## Contributing

Reasonance is open source and welcomes contributions. Whether you're fixing a typo or building a new input modality, you're making development more accessible for everyone.

---

## License

[MIT](LICENSE) — free to use, modify, and distribute.

---

<p align="center">
  <em>Reasonance — because your tools should keep up with your thinking.</em>
</p>
