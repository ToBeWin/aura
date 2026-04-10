# Aura 开发指南

## 项目架构

```
aura/
├── src/                          # React 前端
│   ├── App.tsx                  # 主应用组件
│   ├── App.css                  # 样式
│   └── main.tsx                 # 入口
├── src-tauri/                   # Rust 后端
│   ├── src/
│   │   ├── lib.rs              # Tauri 命令和应用入口
│   │   ├── core.rs             # AuraCore 核心精炼逻辑
│   │   ├── models.rs           # 数据模型定义
│   │   ├── errors.rs           # 错误类型定义
│   │   ├── llm/                # LLM 模块
│   │   │   ├── mod.rs          # 模块导出
│   │   │   ├── client.rs       # Ollama HTTP 客户端
│   │   │   └── prompts.rs      # Prompt 模板
│   │   ├── processing/         # 处理模块
│   │   │   ├── mod.rs          # 模块导出
│   │   │   ├── denoise.rs      # 去噪模块
│   │   │   ├── structure.rs    # 结构映射模块
│   │   │   └── personalize.rs  # 个性化引擎
│   │   └── storage/            # 数据存储
│   │       ├── mod.rs          # 模块导出
│   │       ├── context_store.rs # SQLite 用户上下文
│   │       └── vector_db.rs    # LanceDB 向量数据库（占位符）
│   └── Cargo.toml              # Rust 依赖
└── package.json                # Node.js 依赖
```

## 核心流程

### 文本精炼流程

```
用户输入
  ↓
[DeNoisingModule]
  - 规则去噪（填充词、重复短语）
  - LLM 语义去噪
  ↓
[StructureMapper]
  - 格式推断（如果未指定）
  - 格式转换（邮件、周报等）
  - 语气应用
  ↓
[PersonalizationEngine]
  - 加载用户上下文
  - 应用名称映射
  - 应用地点偏好
  - 应用术语偏好
  - 过滤禁忌词
  - LLM 最终优化
  ↓
精炼输出
```

## 开发命令

### 前端开发
```bash
npm run dev              # 启动 Vite 开发服务器
npm run build            # 构建前端生产版本
npm run preview          # 预览生产构建
```

### Tauri 开发
```bash
npm run tauri dev        # 启动 Tauri 开发模式（前端 + 后端）
npm run tauri build      # 构建生产版本
```

### Rust 开发
```bash
cd src-tauri
cargo check              # 快速检查编译错误
cargo build              # 构建 debug 版本
cargo build --release    # 构建 release 版本
cargo test               # 运行测试
cargo clippy             # 运行 linter
cargo fmt                # 格式化代码
```

## 添加新功能

### 1. 添加新的 Tauri Command

**后端 (src-tauri/src/lib.rs)**:
```rust
#[tauri::command]
async fn my_new_command(param: String) -> Result<String, String> {
    // 实现逻辑
    Ok("result".to_string())
}

// 在 run() 函数中注册
.invoke_handler(tauri::generate_handler![
    greet,
    initialize_aura,
    refine_text,
    my_new_command,  // 添加这里
])
```

**前端 (src/App.tsx)**:
```typescript
import { invoke } from "@tauri-apps/api/core";

async function callMyCommand() {
  const result = await invoke<string>("my_new_command", {
    param: "value"
  });
  console.log(result);
}
```

### 2. 添加新的处理模块

1. 在 `src-tauri/src/processing/` 创建新文件
2. 定义模块结构体和方法
3. 在 `src-tauri/src/processing/mod.rs` 导出
4. 在 `AuraCore` 中集成

### 3. 添加新的数据模型

在 `src-tauri/src/models.rs` 中定义:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyNewModel {
    pub field1: String,
    pub field2: i32,
}
```

## 调试技巧

### 前端调试
- 打开开发者工具: `Cmd+Option+I` (macOS)
- 查看控制台日志
- 使用 React DevTools

### 后端调试
- 使用 `log::info!()`, `log::debug!()` 等宏
- 查看终端输出
- 使用 `RUST_LOG=debug npm run tauri dev` 启用详细日志

### Ollama 调试
```bash
# 查看 Ollama 日志
tail -f ~/.ollama/logs/server.log

# 测试 API
curl http://localhost:11434/api/generate -d '{
  "model": "qwen3.5:2b",
  "prompt": "Hello",
  "stream": false
}'
```

## 性能优化建议

1. **模型预加载**: 在应用启动时预加载 LLM 模型
2. **缓存**: 缓存常用的 LLM 响应
3. **并行处理**: 使用 tokio 并行执行独立任务
4. **模型量化**: 使用量化模型减少内存占用
5. **增量更新**: 只更新变化的 UI 部分

## 测试策略

### 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_denoise() {
        let module = DeNoisingModule::new(/* ... */);
        // 测试逻辑
    }
}
```

### 集成测试
在 `src-tauri/tests/` 目录创建集成测试

### 端到端测试
使用 Tauri 的测试工具进行 UI 自动化测试

## 常见问题

### Q: 如何更换 LLM 模型？
A: 修改 `App.tsx` 中的 `modelName` 参数，确保模型已通过 `ollama pull` 下载。

### Q: 如何添加新的输出格式？
A: 在 `StructureMapper` 中添加新的格式转换逻辑，更新 Prompt 模板。

### Q: 如何实现新的个性化规则？
A: 在 `PersonalizationEngine` 中添加新的规则应用逻辑，更新 `UserContext` 数据模型。

### Q: 如何优化处理速度？
A: 使用更小的模型（如 qwen3.5:0.8b），启用模型量化，减少 Prompt 长度。

## 贡献指南

1. Fork 项目
2. 创建功能分支: `git checkout -b feature/my-feature`
3. 提交更改: `git commit -am 'Add my feature'`
4. 推送分支: `git push origin feature/my-feature`
5. 提交 Pull Request

## 下一步开发任务

参考 `.kiro/specs/voice-text-refinement-engine/tasks.md` 查看完整任务列表。

优先级任务:
1. ✅ 项目初始化和基础 UI
2. ✅ LLM 集成和核心处理模块
3. ⏳ ASR 语音识别集成 (Task 3)
4. ⏳ 纠正历史学习 (Task 12)
5. ⏳ 高级 UI 交互 (Task 13)

## 资源链接

- [Tauri 文档](https://tauri.app/v2/guides/)
- [Rust 文档](https://doc.rust-lang.org/)
- [React 文档](https://react.dev/)
- [Ollama API](https://github.com/ollama/ollama/blob/main/docs/api.md)
