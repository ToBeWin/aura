<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Aura Logo" width="128" height="128">
</p>

# Aura - 智能语音润色引擎

[English](README.md)

Aura 是一个开源、免费、隐私优先的语音润色工具。核心交互是"胶囊模式"：按下快捷键开始录音，再按一次结束，Aura 会自动转写、润色你的语音，并将结果插入到当前输入框（或复制到剪贴板）。

Aura 受 Typeless 交互模式启发，但采用独立实现和产品定位。

![screenshot](assets/1.png)
![screenshot](assets/2.png)
![screenshot](assets/output.gif)

## 核心特性

- **快速捕获**：一个快捷键开始，一个快捷键结束
- **智能润色**：将口语草稿转化为结构化、可直接使用的正式文本
- **自动输入**：直接写入焦点输入框或复制到剪贴板
- **多语言支持**：主要测试了简体中文和英文；其他语言取决于模型能力
- **本地或云端**：ASR 和 LLM 可本地运行或使用云服务
- **极简界面**：主界面仅用于设置、模型管理和历史记录
- **多模式输出**：支持笔记、邮件、报告、社交媒体等不同场景
- **资源自适应**：根据系统资源自动选择最优模型
- **跨平台**：基于 Tauri 2.0 构建，支持 macOS、Windows 和 Linux（Windows/Linux 尚未充分测试）

## 工作原理

1. **快捷键**：`Option + Shift + Space`（macOS）
2. **胶囊流程**：
   - 第一次按下：开始录音
   - 第二次按下：停止并处理
3. **处理流程**：
   - 音频采集 → 语音识别（ASR）→ 去噪处理 → LLM 润色 → 结构化输出
4. **自动输入**：
   - 光标在输入框 → 自动粘贴
   - 否则 → 复制到剪贴板
5. **取消操作**：按 `Esc` 键可随时取消当前流程

## 开发环境设置

```bash
cd aura
npm install
npm run tauri -- dev
```

快速启动：

```bash
./start.sh
```

## 系统要求

- **Rust** 1.70+
- **Node.js** 18+
- **ffmpeg**（音频转换）
- **Ollama**（仅本地 LLM 润色需要）

### 平台支持

- **macOS**：已充分测试，完全支持
- **Windows**：功能可用，但未充分测试
- **Linux**：功能可用，但未充分测试

> 注意：本应用基于 Tauri 2.0 构建，设计上是跨平台的。Windows 和 Linux 版本应该可以正常工作，但尚未经过全面测试。某些平台特定功能（如自动粘贴）在不同平台上的行为可能有所不同。

### Ollama 安装示例

```bash
brew install ollama
ollama serve
ollama pull qwen3.5:2b
```

## 功能特性

### 1. 语音识别（ASR）

**本地选项：**
- Whisper 模型：tiny、base、small、medium、large-v3
- 自动从 HuggingFace 镜像下载模型
- 主要测试了中文和英文；其他语言基于 Whisper 能力支持
- 针对 16kHz 单声道音频优化

**云端提供商：**
- OpenAI（Whisper API、GPT-4o 转写）
- Groq（Whisper large-v3-turbo）
- Deepgram（Nova-2）
- AssemblyAI
- Azure 语音服务
- Google Speech-to-Text
- 自定义 OpenAI 兼容端点

### 2. 文本润色（LLM）

**处理流程：**
- **去噪**：移除填充词（"嗯"、"啊"、"那个"、"um"、"uh"）
- **上下文应用**：用户术语、名称映射、禁用词
- **LLM 润色**：将口语转化为正式文本
- **结构检测**：自动格式化枚举列表
- **语言规范化**：适时转换为简体中文

**本地选项：**
- Ollama 集成（qwen3.5:2b、llama3.2、gemma3、mistral 等）
- 自动模型预加载，加快响应速度
- 资源感知的模型选择

**云端提供商：**
- OpenAI（GPT-4.1、GPT-4o）
- Anthropic（Claude 3.5 Sonnet、Haiku）
- Google Gemini（1.5 Pro、2.0 Flash）
- DeepSeek
- Qwen（阿里云通义千问）
- GLM（智谱 AI）
- Kimi（月之暗面）
- Minimax
- OpenRouter
- 自定义 OpenAI 兼容端点

### 3. 输出模式

- **笔记**（默认）：自然、清晰的笔记文本
- **邮件**：简洁、专业的邮件正文
- **报告**：结构化的简短汇报，适合日报/周报
- **社交**：适合社交平台发布的短文案

### 4. 语言支持

**已充分测试：**
- 简体中文
- 英文

**其他语言：**
- 基于底层模型能力支持（ASR 使用 Whisper，润色使用所选 LLM）
- 未经过全面测试，功能可能有差异
- 语言检测和处理取决于模型性能

## 设置说明

桌面界面提供全面的配置选项：

- **通用**：界面语言（中文/英文）、音频输入设备
- **语音识别**：本地/云端路由、提供商选择、模型配置
- **润色模型**：LLM 提供商设置、推荐模型选择
- **唤醒词**：启用/禁用唤醒词检测（开发中）
- **历史记录**：分页查看最近的转写和润色结果
- **诊断**：实时环境健康检查（ASR、LLM、输入、运行时）

### 推荐模型

**本地 ASR：**
- **whisper-base**：推荐默认，速度和准确度平衡（~142MB）
- **whisper-tiny**：最低资源占用，适合老机器（~75MB）
- **whisper-small**：更高精度，下载较大（~466MB）

**本地 LLM：**
- **qwen3.5:2b**：推荐默认，快速高效
- **llama3.2:3b**：英文场景的好选择
- **gemma3:4b**：性能均衡

**云端 ASR：**
- **OpenAI**：gpt-4o-mini-transcribe（快速且经济）
- **Groq**：whisper-large-v3-turbo（最快）
- **Deepgram**：nova-2（高准确度）

**云端 LLM：**
- **OpenAI**：gpt-4.1-mini（快速且性价比高）
- **Anthropic**：claude-3-5-sonnet-latest（最高质量）
- **DeepSeek**：deepseek-chat（中文支持优秀）

## 自动输入

**macOS：**
自动粘贴需要辅助功能权限。如果未授予权限，Aura 将回退到剪贴板输出。

在此处启用：`系统设置 → 隐私与安全性 → 辅助功能`

**Windows/Linux：**
自动粘贴行为可能与 macOS 不同。应用会尝试粘贴，或根据系统能力回退到剪贴板。

## 构建与发布

### 构建应用

```bash
npm run build
```

### macOS 发布构建

```bash
./build-release.sh
```

主要输出：
- `Aura-macos-universal.dmg`

### macOS 签名与公证

对于公开分发，需配置 Apple Developer 证书并遵循：

- `RELEASE_CHECKLIST.md`
- `build-release.sh`

## 项目结构

```
aura/
├── src/                    # 前端（React + TypeScript）
│   ├── App.tsx            # 主应用组件
│   ├── components/        # UI 组件
│   └── assets/            # 前端资源
├── src-tauri/             # 后端（Rust）
│   ├── src/
│   │   ├── asr/          # 语音识别模块
│   │   ├── llm/          # LLM 客户端
│   │   ├── processing/   # 文本处理（去噪）
│   │   ├── storage/      # 数据存储
│   │   ├── learning/     # 学习与纠错
│   │   ├── core.rs       # 核心润色引擎
│   │   ├── models.rs     # 数据模型
│   │   ├── settings.rs   # 配置管理
│   │   └── lib.rs        # Tauri 命令
│   ├── icons/            # 应用图标
│   └── Cargo.toml        # Rust 依赖
├── scripts/               # 发布工具脚本
├── package.json          # Node.js 依赖
└── README.md             # 项目文档
```

### 核心架构

**前端（React + TypeScript）：**
- 胶囊 UI 状态管理
- 设置界面和历史记录
- 音频录制（WebRTC）
- 实时音频电平显示

**后端（Rust + Tauri）：**
- **ASR 引擎**：本地 Whisper 和云端 API 集成
- **LLM 客户端**：Ollama 和多云端提供商支持
- **核心引擎**：去噪 → 上下文应用 → LLM 润色 → 结构化
- **存储层**：SQLite（用户上下文）+ LanceDB（向量检索）
- **系统集成**：全局快捷键、辅助功能 API、剪贴板

## 技术栈

- **前端**：React 19、TypeScript、Vite
- **后端**：Rust、Tauri 2.0
- **语音识别**：whisper-rs、云端 API
- **LLM**：Ollama、OpenAI、Anthropic、Gemini 等
- **音频处理**：cpal、hound、ffmpeg
- **数据库**：rusqlite、LanceDB
- **系统 API**：macOS Accessibility、Core Graphics

## 路线图

**当前版本（v0.1.0）：**
- ✅ 端到端语音流程
- ✅ 本地 + 云端双栈
- ✅ Typeless 风格的胶囊交互
- ✅ 多输出模式
- ✅ 资源自适应模型选择

**计划中：**
- 🔄 唤醒词实时监听
- 🔄 Windows 和 Linux 全面测试
- 🔄 增强的提供商自动检测
- 🔄 用户上下文和纠错记忆（个性化功能）
- 🔄 更多输出模式和场景
- 🔄 多语言测试和优化

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request！

## 致谢

- 灵感来源：Typeless
- 语音识别：OpenAI Whisper
- LLM 支持：Ollama、OpenAI、Anthropic 等
