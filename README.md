# Aura - Typeless-Style Voice-to-Text Refinement Engine

Aura is an open-source, free, privacy-first voice refinement tool. Its core experience is the capsule: press a hotkey to record, press again to finish, then Aura transcribes and refines your speech and inserts the result into the active input field (or falls back to clipboard).

Aura is inspired by the Typeless interaction model, but built independently with its own implementation and product identity.

## Highlights

- **Fast capture**: one hotkey to start, one to finish
- **Auto refinement**: turns spoken drafts into polished, structured text
- **Auto insert**: writes into the focused input field or clipboard
- **Bilingual**: Simplified Chinese and English
- **Local or cloud**: ASR and LLM can run locally or via providers
- **Minimal UI**: main app is settings + history only

## How It Works

1. **Hotkey**: `Option + Shift + Space`
2. **Capsule flow**:
   - First press: start recording
   - Second press: stop and process
3. **Auto insert**:
   - Cursor in input field → auto paste
   - Otherwise → clipboard

## Development Setup

```bash
cd aura
npm install
npm run tauri -- dev
```

Quick start:

```bash
./start.sh
```

## Requirements

- **Rust** 1.70+
- **Node.js** 18+
- **ffmpeg** (audio conversion)
- **Ollama** (only for local refinement)

### Ollama Example

```bash
brew install ollama
ollama serve
ollama pull qwen3.5:2b
```

## Features

### 1. Speech Recognition (ASR)

- Local: Whisper (tiny/base/small/medium/large-v3)
- Cloud providers:
  - OpenAI
  - Groq
  - Deepgram
  - AssemblyAI
  - Azure Speech
  - Google Speech-to-Text
  - Custom compatible

### 2. Text Refinement (LLM)

- Local: Ollama
- Cloud providers:
  - OpenAI
  - Anthropic
  - Gemini
  - DeepSeek
  - Qwen
  - GLM
  - Kimi
  - Minimax
  - OpenRouter
  - Custom compatible

## Settings

The desktop UI is for configuration and history:

- ASR / LLM local vs cloud routing
- Provider selection + recommended model dropdowns
- ASR language preference (auto / Chinese / English)
- Recent history (paginated)
- Status and diagnostics

## Auto Insert (Important)

Auto paste requires macOS Accessibility permission. If it is not granted, Aura will fall back to clipboard output.

Enable it here:

```
System Settings → Privacy & Security → Accessibility
```

## Build & Release

### Build the app

```bash
npm run build
```

### macOS release build

```bash
./build-release.sh
```

Primary output:
- `Aura-macos-universal.dmg`

### macOS signing & notarization

For public distribution, configure Apple Developer certificates and follow:

- `RELEASE_CHECKLIST.md`
- `build-release.sh`

## Project Structure

- `src/` Frontend (React + TS)
- `src-tauri/` Backend (Rust)
- `scripts/` Release tooling

## Roadmap (v1.0.0)

Current:
- End-to-end voice flow
- Local + cloud dual stack
- Typeless-style capsule interaction

Next:
- Stronger provider auto-detection
- More curated model recommendations
- Official Windows / Linux releases

## Maintainer

Maintainer: **ToBeWin**  
Email: **jingyecn@gmail.com**

## License

MIT
