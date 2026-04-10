# Changelog

All notable changes to Aura will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-01

### Added

#### Core Features
- 🎤 Local voice-to-text refinement engine
- 🧹 Intelligent text denoising (filler word removal, phrase merging)
- 📝 Automatic format detection and conversion (Email, Weekly Report, Social Media, Code Comment)
- 🎨 Tone adjustment (Professional, Casual, Friendly, Formal)
- 🔒 Complete offline operation with privacy-first design

#### AI & Processing
- Local LLM integration via Ollama (qwen3.5:2b)
- ASR engine framework (whisper.cpp ready)
- Semantic denoising with confidence scoring
- Structure mapping with metadata extraction
- Personalization engine with user context

#### User Interface
- Modern gradient UI with dark theme
- Interactive text editing (click words for alternatives)
- Voice command support ("更正式一点", "简短一些")
- Undo/Redo functionality
- Real-time processing indicators
- Confidence and processing time display

#### Learning & Personalization
- Correction history tracking and learning
- User context management (name mappings, location preferences, terminology)
- Forbidden words filtering
- Automatic pattern extraction from corrections
- Vector-based correction retrieval

#### Data Management
- SQLite for structured data (user context)
- LanceDB for vector storage (correction history)
- Import/Export user context (JSON)
- Correction history viewer

#### Performance
- Model preloading on startup
- Resource monitoring and auto-downgrade
- Optimal model selection based on available memory
- Async processing with tokio

#### Cross-Platform
- Windows support (.exe, .msi installers)
- macOS support (.dmg, .app bundles)
- Linux support (.deb, .rpm, AppImage)
- System tray integration
- Native notifications

### Technical Details

**Frontend**:
- React 18 + TypeScript
- Vite for build tooling
- Tauri IPC for backend communication

**Backend**:
- Rust with Tauri 2.0
- tokio for async runtime
- reqwest for HTTP client
- rusqlite for SQLite
- sysinfo for resource monitoring

**AI Models**:
- Ollama with qwen3.5 models (0.8b, 2b, 9b)
- Local inference (no cloud API)
- Embedding generation for vector search

### Documentation

- README with quick start guide
- QUICKSTART guide with examples
- DEVELOPMENT guide for contributors
- USAGE_GUIDE with detailed scenarios
- TEST_EXAMPLES with test cases
- TESTING checklist
- ARCHITECTURE documentation
- CONTRIBUTING guidelines

### Known Limitations

- ASR engine is placeholder (requires whisper.cpp integration)
- Voice command uses text input (requires real voice recognition)
- No real-time streaming transcription yet
- Limited to desktop platforms (mobile coming later)

### Future Roadmap

- Real-time voice input streaming
- Mobile app versions (iOS, Android)
- Plugin system for custom processors
- Cloud sync (optional, privacy-preserving)
- Multi-language support expansion
- Collaborative features (share templates)

---

## [Unreleased]

### Planned for 0.2.0

- Real whisper.cpp integration for ASR
- Real-time voice command recognition
- Streaming transcription
- Performance benchmarks
- Automated testing suite

---

**Note**: This is the initial release. We welcome feedback and contributions!
