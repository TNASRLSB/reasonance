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

**Built for every human.** Not an afterthought — accessibility is the architecture. Screen readers, keyboard navigation, high contrast, reduced motion. Same power, every modality. WCAG 2.1 AA compliant.

**Secure by design.** API keys never touch the browser. All LLM calls are proxied through the native Rust backend. Markdown output is sanitized against XSS.

**Native speed, tiny footprint.** Built on Tauri + Svelte. Sub-100 MB memory. Starts in under a second. No Electron. No bloat.

---

## Features

### Multi-AI Terminal
Run Claude, Gemini, GPT, or local models side by side. Each in its own tab, each with full context. Switch between them like browser tabs — compare approaches, pick what works.

### Real-Time Context Awareness
See exactly how much context your AI has left. A live progress bar shows token usage so you never hit the wall mid-thought.

### Brutalist Code Editor
CodeMirror 6 with a distinctive brutalist theme. Fast, precise, no distractions. Syntax highlighting, intelligent completions, and a look that means business.

### Smart File Tree
Your project at a glance. Navigate, open, explore — all keyboard-accessible, all instant.

### System-Aware Theming
Reasonance reads your desktop colors and adapts. Your IDE, your aesthetic.

### AI Context Menu
Select code, right-click, and ask any connected AI to explain, refactor, or extend it. No copy-paste. No context switching.

### Built-In Diff View
Side-by-side comparison with accept/reject controls. Review AI-generated changes before they touch your code.

### Accessibility-First Design
- **Atkinson Hyperlegible** font family — designed for maximum readability
- High contrast, large touch targets, keyboard-navigable everything
- Screen reader compatible with full ARIA support
- Reduced motion support for seizure safety

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
| Typography | **Atkinson Hyperlegible** | Designed for readability, not aesthetics alone |

---

## Roadmap

- [ ] Multi-modal input (voice, switch, eye tracking)
- [ ] Visual dataflow editor for AI orchestration
- [ ] Persistent decision log across sessions
- [ ] Plugin ecosystem with self-evolving skills
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
