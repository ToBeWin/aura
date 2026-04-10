# Aura 快速开始指南

## 🚀 快速启动

### 1. 确保 Ollama 正在运行

```bash
# 检查 Ollama 是否运行
curl http://localhost:11434/api/tags

# 如果没有运行，启动它
ollama serve
```

### 2. 启动 Aura 开发模式

```bash
cd aura
npm run tauri dev
```

应用将自动打开，等待几秒钟完成初始化。

## 💡 使用示例

### 示例 1: 口语转专业邮件

**输入**:
```
呃，小李啊，那个...明天上海那个会，你帮我准备个 PPT，主讲是苏，别忘了带那个 10% 的折价券...
```

**设置**:
- 格式: Email
- 语气: Professional

**点击 "Refine Text"**，等待处理完成。

### 示例 2: 随意想法转周报

**输入**:
```
这周做了啥呢，嗯，主要是修了那个登录的 bug，然后呢，还优化了一下数据库查询，对了，还开了几个会讨论新功能
```

**设置**:
- 格式: Weekly Report
- 语气: Professional

### 示例 3: 自动检测格式

**输入**:
```
今天天气真好，想去公园走走，顺便拍点照片发朋友圈
```

**设置**:
- 格式: Auto Detect Format (会自动识别为 Social Media)
- 语气: Casual

## 🎯 当前功能状态

✅ **已实现**:
- 基础 UI 界面
- 文本去噪（规则 + LLM）
- 格式转换（邮件、周报、社交媒体、代码注释）
- 语气调整
- 个性化引擎（名称映射、地点偏好、术语）
- 本地 LLM 集成（Ollama）
- 数据存储（SQLite + LanceDB 占位符）

⏳ **待实现**:
- 语音录制和 ASR 集成
- 实时语音识别
- 纠正历史学习
- 无打字微调界面
- 系统托盘和全局快捷键

## 🐛 故障排除

### 应用无法启动
- 确保 Ollama 正在运行: `ollama serve`
- 确保 qwen3.5:2b 模型已下载: `ollama pull qwen3.5:2b`
- 检查端口 11434 是否被占用

### 处理失败
- 检查 Ollama 日志
- 确保输入文本不为空且小于 10000 字符
- 尝试重启应用

### 编译错误
- 运行 `./test-backend.sh` 检查环境
- 确保 Rust 版本 >= 1.70
- 运行 `cargo clean` 然后重新编译

## 📝 下一步

1. 测试当前功能，提供反馈
2. 实现 ASR 语音识别（Task 3）
3. 实现纠正历史学习（Task 12）
4. 实现高级 UI 交互（Task 13）

## 🔗 相关资源

- [Tauri 文档](https://tauri.app)
- [Ollama 文档](https://ollama.ai)
- [项目规格文档](.kiro/specs/voice-text-refinement-engine/)
