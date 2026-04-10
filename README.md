# Aura - Typeless 风格语音到文本精炼引擎

Aura 是一款开源、免费、隐私优先的语音到文本精炼工具。它以“胶囊”交互为核心：通过快捷键唤起录音，结束后自动转写与润色，并将结果直接输入到当前输入框（或回退到剪贴板）。

本项目定位为 Typeless 风格的开源替代，但保持独立实现与独特体验。

## 亮点

- **极速输入**：快捷键唤起胶囊，开始说话，再按一次结束
- **自动润色**：将口语整理为书面化、结构化文本
- **自动输入**：优先写入当前输入框，否则写入剪贴板
- **双语支持**：中文（简体）与英文都可稳定工作
- **本地 / 云端**：ASR 与润色模型均支持本地或云端
- **极简 UI**：主界面仅用于设置与历史查看

## 体验方式

1. **快捷键**：`Option + Shift + Space`
2. **胶囊模式**：
   - 第一次按下：开始录音
   - 第二次按下：结束录音并处理
3. **自动写入**：
   - 光标在输入框 → 自动写入
   - 否则 → 写入剪贴板

## 安装与运行（开发模式）

```bash
cd aura
npm install
npm run tauri -- dev
```

如果你只想快速体验，推荐运行：

```bash
./start.sh
```

## 依赖要求

- **Rust** 1.70+
- **Node.js** 18+
- **ffmpeg**（用于音频格式转换）
- **Ollama**（仅本地润色时需要）

### Ollama 示例

```bash
brew install ollama
ollama serve
ollama pull qwen3.5:2b
```

## 功能详解

### 1. 语音识别（ASR）

- 本地：Whisper（tiny/base/small/medium/large-v3）
- 云端（已支持）：
  - OpenAI
  - Groq
  - Deepgram
  - AssemblyAI
  - Azure Speech
  - Google Speech-to-Text
  - Custom compatible

### 2. 文本润色（LLM）

- 本地：Ollama
- 云端（已支持）：
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

## 设置说明

主界面用于以下配置：

- ASR 与润色模型的 **本地 / 云端切换**
- 云端 Provider 选择 + 推荐模型下拉
- ASR 语言偏好（自动 / 中文 / 英文）
- 最近历史记录查看（分页）
- 基础状态与诊断

## 自动输入（重要）

自动写入需要 macOS 的**辅助功能权限**。若未授权，Aura 会自动回退到剪贴板。

你可以在系统设置中打开：

```
System Settings → Privacy & Security → Accessibility
```

## 构建与发布

### 构建本地应用

```bash
npm run build
```

### macOS 发布打包

```bash
./build-release.sh
```

生成的主产物：
- `Aura-macos-universal.dmg`（Universal）

### macOS 正式签名与公证

如需公开分发，请配置 Apple Developer 证书并执行签名流程，详情见：

- `RELEASE_CHECKLIST.md`
- `build-release.sh`

## 项目结构

- `src/` 前端界面（React + TS）
- `src-tauri/` 后端逻辑（Rust）
- `scripts/` 发布打包脚本

## Roadmap（v1.0.0）

当前版本已覆盖：
- 完整端到端语音输入流程
- 本地与云端双栈
- Typeless 风格的“胶囊”交互

后续计划：
- 更丰富的云端 provider 自动识别能力
- 更细粒度模型推荐
- 面向 Windows / Linux 的正式打包与发布

## 维护者

Maintainer: **ToBeWin**  
Email: **jingyecn@gmail.com**

## License

MIT
