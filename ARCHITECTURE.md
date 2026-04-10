# Aura Architecture Documentation

## Overview

Aura is a desktop application built with Tauri 2.0, combining a Rust backend with a React frontend. The architecture follows a modular design with clear separation of concerns.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend (React)                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   App    │  │ Context  │  │ History  │  │  Audio   │   │
│  │Component │  │ Manager  │  │  Modal   │  │  Upload  │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       │             │              │             │          │
│       └─────────────┴──────────────┴─────────────┘          │
│                          │                                   │
│                    Tauri IPC                                 │
│                          │                                   │
└──────────────────────────┼───────────────────────────────────┘
                           │
┌──────────────────────────┼───────────────────────────────────┐
│                    Backend (Rust)                            │
│                          │                                   │
│  ┌───────────────────────┴────────────────────────┐         │
│  │              AuraCore (Orchestrator)           │         │
│  └───┬────────────┬────────────┬──────────────┬───┘         │
│      │            │            │              │             │
│  ┌───▼────┐  ┌───▼────┐  ┌───▼────┐  ┌──────▼──────┐      │
│  │Denoise │  │Structure│  │Personal│  │  Learning   │      │
│  │ Module │  │ Mapper  │  │ Engine │  │  Manager    │      │
│  └───┬────┘  └───┬────┘  └───┬────┘  └──────┬──────┘      │
│      │            │            │              │             │
│      └────────────┴────────────┴──────────────┘             │
│                          │                                   │
│  ┌───────────────────────┴────────────────────────┐         │
│  │              LocalLLM (Ollama Client)          │         │
│  └───────────────────────┬────────────────────────┘         │
│                          │                                   │
│  ┌───────────────────────┴────────────────────────┐         │
│  │         Storage Layer                          │         │
│  │  ┌──────────────┐  ┌──────────────┐           │         │
│  │  │UserContext   │  │LocalVectorDB │           │         │
│  │  │Store(SQLite) │  │  (LanceDB)   │           │         │
│  │  └──────────────┘  └──────────────┘           │         │
│  └────────────────────────────────────────────────┘         │
│                                                              │
│  ┌────────────────────────────────────────────────┐         │
│  │         External Services (Local)              │         │
│  │  ┌──────────────┐  ┌──────────────┐           │         │
│  │  │   Ollama     │  │  ASR Engine  │           │         │
│  │  │(localhost:   │  │(whisper.cpp) │           │         │
│  │  │   11434)     │  │              │           │         │
│  │  └──────────────┘  └──────────────┘           │         │
│  └────────────────────────────────────────────────┘         │
└──────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. AuraCore (Orchestrator)

**Location**: `src-tauri/src/core.rs`

**Responsibilities**:
- Orchestrates the entire refinement pipeline
- Coordinates between denoise, structure mapping, and personalization
- Handles error recovery and fallback logic
- Tracks processing time and confidence scores

**Key Methods**:
- `new()` - Initialize with LLM, storage, and processing modules
- `refine_thought()` - Main entry point for text refinement

### 2. DeNoisingModule

**Location**: `src-tauri/src/processing/denoise.rs`

**Responsibilities**:
- Remove filler words (呃, 那个, um, uh)
- Merge repeated phrases
- Clean up extra whitespace
- Preserve semantic meaning

**Algorithm**:
1. Rule-based filtering (regex patterns)
2. LLM-based semantic cleaning
3. Confidence scoring

### 3. StructureMapper

**Location**: `src-tauri/src/processing/structure.rs`

**Responsibilities**:
- Detect output format (email, report, social media, code comment)
- Apply format-specific templates
- Apply tone adjustments (professional, casual, friendly, formal)
- Extract metadata (subject, recipients, etc.)

**Supported Formats**:
- Email: Subject, greeting, body, signature
- Weekly Report: Time period, sections, bullet points
- Social Media: Hashtags, mentions, emoji
- Code Comment: Docstring, inline comments

### 4. PersonalizationEngine

**Location**: `src-tauri/src/processing/personalize.rs`

**Responsibilities**:
- Apply user context (name mappings, location preferences)
- Retrieve and apply correction history
- Filter forbidden words
- Apply terminology preferences

**Data Sources**:
- UserContextStore (SQLite)
- LocalVectorDB (LanceDB) for correction history
- LLM for final optimization

### 5. LocalLLM

**Location**: `src-tauri/src/llm/client.rs`

**Responsibilities**:
- Interface with Ollama API (localhost:11434)
- Generate text completions
- Generate embeddings for vector search
- Model preloading and caching

**API Methods**:
- `generate()` - Text generation with prompt
- `embed()` - Generate embedding vector
- `preload()` - Warm up model
- `check_model_available()` - Verify model exists

### 6. Storage Layer

#### UserContextStore (SQLite)

**Location**: `src-tauri/src/storage/context_store.rs`

**Schema**:
```sql
CREATE TABLE user_context (
    user_id TEXT PRIMARY KEY,
    name_mappings TEXT,  -- JSON
    location_preferences TEXT,  -- JSON
    terminology_preferences TEXT,  -- JSON
    forbidden_words TEXT,  -- JSON
    custom_rules TEXT,  -- JSON
    created_at TEXT,
    updated_at TEXT
);
```

#### LocalVectorDB (LanceDB)

**Location**: `src-tauri/src/storage/vector_db.rs`

**Collections**:
- `correction_history` - Stores correction records with embeddings

**Operations**:
- Insert correction with embedding
- Search similar corrections by vector similarity
- Filter by user_id

### 7. CorrectionManager

**Location**: `src-tauri/src/learning/correction.rs`

**Responsibilities**:
- Save user corrections
- Extract correction patterns
- Generate embeddings for corrections
- Retrieve similar corrections for new inputs

**Workflow**:
1. User edits output → Save correction
2. Extract pattern (original → corrected)
3. Generate embedding of original text
4. Store in LanceDB
5. On new input → Search similar corrections → Apply patterns

### 8. ResourceMonitor

**Location**: `src-tauri/src/monitoring.rs`

**Responsibilities**:
- Monitor system memory and CPU usage
- Suggest optimal model based on resources
- Trigger model downgrade if resources constrained

**Thresholds**:
- Memory: 500MB available
- CPU: 80% usage
- Model selection: 0.8b (low) / 2b (medium) / 9b (high)

## Data Flow

### Text Refinement Flow

```
User Input
    │
    ▼
┌─────────────────┐
│ Input Validation│
│ (length, format)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Denoise Module  │
│ - Remove fillers│
│ - Merge repeats │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Structure Mapper│
│ - Detect format │
│ - Apply template│
│ - Apply tone    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│Personalization  │
│ - User context  │
│ - Corrections   │
│ - Forbidden words│
└────────┬────────┘
         │
         ▼
    Refined Output
```

### Correction Learning Flow

```
User Edits Output
    │
    ▼
┌─────────────────┐
│ Extract Pattern │
│ (diff analysis) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│Generate Embedding│
│ (LLM embed API) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Store in LanceDB│
│ (with metadata) │
└─────────────────┘

Next Input
    │
    ▼
┌─────────────────┐
│Search Similar   │
│ (vector search) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Apply Patterns  │
│ (personalize)   │
└─────────────────┘
```

## Frontend Architecture

### Component Hierarchy

```
App
├── Header
│   ├── Status Indicator (Online/Offline)
│   ├── History Button
│   └── Settings Button
├── Controls
│   ├── Format Selector
│   └── Tone Selector
├── Editor Container
│   ├── Input Section
│   │   ├── Text Area
│   │   └── Audio Upload
│   └── Output Section
│       ├── Output Text (Interactive)
│       ├── Alternatives Popup
│       ├── Edit Mode
│       └── Metadata Display
├── Action Buttons
│   ├── Refine Button
│   ├── History Button
│   └── Voice Command Button
├── History Modal
│   └── Correction Records List
└── Context Manager Modal
    ├── Name Mappings
    ├── Location Preferences
    ├── Terminology Preferences
    └── Forbidden Words
```

### State Management

Uses React hooks for local state:
- `useState` for component state
- `useEffect` for side effects (initialization, network monitoring)
- No global state management (Redux/MobX) - keeps it simple

### IPC Communication

**Frontend → Backend**:
```typescript
invoke("initialize_aura", { modelName, dbPath, vectorDbPath })
invoke("refine_text", { request: { text, user_id, output_format, tone } })
invoke("save_correction", { request: { user_id, original_text, corrected_text, context } })
invoke("get_correction_suggestions", { userId, text })
invoke("get_user_context", { userId })
invoke("update_user_context", { context })
```

**Backend → Frontend**:
- Currently no events, but can add progress events in future

## Performance Optimizations

### 1. Model Preloading

- Model is preloaded in background on app startup
- First request is faster (no cold start)
- Implemented in `LocalLLM::preload()`

### 2. Resource Monitoring

- Monitors system memory and CPU
- Auto-selects optimal model size
- Downgrades model if resources constrained

### 3. Caching

- User context cached in memory
- Correction history cached after first retrieval
- LLM responses not cached (always fresh)

### 4. Async Processing

- All I/O operations are async (tokio)
- Non-blocking UI during processing
- Progress indicators for long operations

## Security Considerations

### 1. Privacy

- All processing is local (no external API calls)
- No telemetry or analytics
- User data never leaves the device

### 2. Data Storage

- SQLite database stored locally
- LanceDB vector store stored locally
- Optional encryption for sensitive data (future)

### 3. Input Validation

- Length limits (10,000 characters)
- Format validation
- SQL injection prevention (parameterized queries)

## Error Handling

### Error Types

```rust
pub enum AuraError {
    InputValidation { message, error_code },
    Processing { message, error_code },
    Storage { message, error_code },
    Network { message, error_code },
}
```

### Fallback Strategy

1. **Denoise fails** → Use original text
2. **Structure mapping fails** → Use denoised text
3. **Personalization fails** → Use structured text
4. **LLM unavailable** → Return error to user

### Logging

- Uses `log` crate with `env_logger`
- Levels: ERROR, WARN, INFO, DEBUG
- Logs stored in system log directory

## Testing Strategy

### Unit Tests

- Test individual modules in isolation
- Mock LLM responses
- Test error handling

### Integration Tests

- Test full pipeline (denoise → structure → personalize)
- Test storage operations
- Test IPC communication

### Property-Based Tests

- Test invariants (e.g., output length ≤ input length * 2)
- Test idempotence (refine(refine(x)) ≈ refine(x))
- Test preservation (semantic meaning preserved)

## Deployment

### Build Process

```bash
# Development
npm run tauri dev

# Production
npm run tauri build
```

### Platform-Specific Builds

- **Windows**: .exe, .msi
- **macOS**: .dmg, .app
- **Linux**: .deb, .rpm, AppImage

### Distribution

- GitHub Releases
- Direct download from website
- Package managers (future: Homebrew, Chocolatey, apt)

## Future Enhancements

1. **Real-time voice input** - Stream audio to ASR
2. **Multi-language support** - Support more languages
3. **Cloud sync** (optional) - Sync settings across devices
4. **Plugin system** - Allow custom processing modules
5. **Mobile apps** - iOS and Android versions
6. **Collaborative features** - Share templates and corrections

## References

- [Tauri Documentation](https://tauri.app/)
- [Ollama API](https://github.com/ollama/ollama/blob/main/docs/api.md)
- [LanceDB Documentation](https://lancedb.github.io/lancedb/)
- [Whisper.cpp](https://github.com/ggerganov/whisper.cpp)
