# Reasonance Documentation

## 1. Introduction

### What is Reasonance
Reasonance is a lightweight desktop IDE designed for vibecoders — developers who work with AI-powered coding assistants. It provides a clean, focused environment with integrated LLM terminals, a code editor, and file management.

### System Requirements
- Linux (KDE Plasma recommended), macOS, or Windows
- At least one LLM CLI tool installed (Claude Code, Ollama, etc.)

### Installation
Download the latest release from the GitHub Releases page. On Linux, install the .deb or .AppImage package.

## 2. Interface

### Layout
Reasonance uses a three-panel layout:
- **Left panel**: File tree for navigating your project
- **Center panel**: Code editor with syntax highlighting
- **Right panel**: LLM terminal for AI-powered coding

### Menu Bar
Access all features from the menu bar:
- **File**: Open folders, manage files, recent projects
- **Edit**: Undo, redo, clipboard, search
- **View**: Theme, readability, panel visibility
- **Terminal**: Create LLM terminals, YOLO mode
- **Git**: Status, commit, push, pull, log
- **Help**: Documentation, keyboard shortcuts

### Status Bar
The bottom status bar shows:
- App name and detected LLM count
- Active terminal session info (context %, model, reset timer, messages)
- Active file info (name, language, encoding)
- YOLO mode indicator (red bar when active)

### Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| Ctrl+P | Quick file search |
| Ctrl+Shift+F | Find in files |
| Ctrl+S | Save file |
| Ctrl+, | Open settings |
| F1 | Open documentation |

## 3. File Management

### Opening a Project
Use **File > Open Folder** or click "Open Folder" on the welcome screen. Recent projects are listed for quick access.

### Navigating Files
Click files in the file tree to open them. Right-click for context menu actions. Use Ctrl+P for quick file search by name.

### Editing Files
Files open in read-only mode by default. Click "Read-only" to toggle editing mode. Changes are tracked with shadow copies for diff detection.

### Search
- **Ctrl+P**: Search files by name
- **Ctrl+Shift+F**: Search file contents (grep)

## 4. LLM Terminal

### Starting an LLM
Click the **+** button in the terminal panel to see available LLMs. Reasonance auto-detects installed CLI tools (Claude Code, Ollama, etc.).

### Multiple Instances
Run multiple LLM sessions simultaneously. Each instance has its own tab. Switch between instances using the tab bar.

### YOLO Mode
Enable YOLO mode from the toolbar or **Terminal > YOLO Mode**. This passes the --dangerously-skip-permissions flag to Claude Code, allowing it to run without confirmation prompts. The status bar turns red as a warning.

### Context Tracking
The status bar displays real-time context window usage parsed from LLM output, including:
- Session usage percentage with visual bar
- Active model name
- Messages remaining
- Reset countdown timer

## 5. Git Integration

Access Git commands from the **Git** menu. Commands run in the active terminal:
- **Status**: Show working tree status
- **Commit**: Start a commit (type your message)
- **Push**: Push to remote
- **Pull**: Pull from remote
- **Log**: Show recent commit history

## 6. Settings

Open settings with **Ctrl+,** or the gear icon.

### Theme
Choose between Light, Dark, or System (follows OS preference). On KDE/Wayland, System mode uses native detection with fallback to dark.

### Language
Select from 9 languages: English, Italiano, Deutsch, Espanol, Francais, Portugues, Zhongwen, Hindi, Al-Arabiya. Arabic enables RTL layout.

### Font & Readability
- Custom font family and size
- Enhanced Readability mode: larger text, increased spacing, optimized for accessibility

### LLM Configuration
LLMs are auto-detected on first launch. Manual configuration via TOML config file for advanced setups.

## 7. Troubleshooting

### LLMs Not Detected
- Ensure the LLM CLI tool is installed and in your PATH
- Try **Terminal > Detect LLMs** to re-scan
- Check the config file for manual configuration

### Blurry Rendering on Linux
Reasonance includes a fix for fractional scaling on KDE/Wayland (WebKitGTK). If rendering is still blurry, check your display scaling settings.

### Theme Not Switching
If the theme doesn't respond to system changes, try setting it explicitly to Light or Dark in Settings, then back to System.

### FAQ
**Q: Can I use multiple LLMs at once?**
A: Yes, each LLM gets its own tab. Click + to add more instances.

**Q: How do I configure a custom LLM?**
A: Edit the TOML config file at ~/.config/reasonance/config.toml

**Q: Does YOLO mode work with all LLMs?**
A: YOLO mode is currently optimized for Claude Code. Other LLMs may have different confirmation mechanisms.
