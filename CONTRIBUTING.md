# Contributing to Aura

感谢你对 Aura 项目的关注！我们欢迎所有形式的贡献。

## 如何贡献

### 报告 Bug

如果你发现了 Bug，请创建一个 Issue，包含以下信息：

- Bug 的详细描述
- 复现步骤
- 预期行为和实际行为
- 系统环境（操作系统、Rust 版本、Node.js 版本）
- 相关日志或截图

### 提出新功能

如果你有新功能的想法，请创建一个 Issue，描述：

- 功能的用途和价值
- 预期的使用场景
- 可能的实现方案

### 提交代码

1. Fork 本仓库
2. 创建你的功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交你的修改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

### 代码规范

#### Rust 代码

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 为公共 API 编写文档注释
- 为新功能编写单元测试

#### TypeScript/React 代码

- 使用 ESLint 和 Prettier 格式化代码
- 使用 TypeScript 类型注解
- 为组件编写清晰的 Props 接口
- 保持组件简洁和可复用

### 提交信息规范

使用清晰的提交信息：

- `feat: 添加新功能`
- `fix: 修复 Bug`
- `docs: 更新文档`
- `style: 代码格式调整`
- `refactor: 代码重构`
- `test: 添加测试`
- `chore: 构建或工具相关`

## 开发环境设置

1. 安装依赖（见 README.md）
2. 运行开发服务器：`npm run tauri dev`
3. 运行测试：`cargo test`
4. 构建生产版本：`npm run tauri build`

## 项目架构

### 后端（Rust）

- `core.rs` - 核心精炼逻辑
- `llm/` - LLM 客户端和 Prompt 模板
- `processing/` - 去噪、结构映射、个性化引擎
- `storage/` - 数据存储（SQLite + LanceDB）
- `asr/` - 语音识别引擎
- `learning/` - 纠正历史和学习

### 前端（React）

- `App.tsx` - 主应用组件
- `components/` - 可复用组件
- `App.css` - 样式

## 测试

### 运行测试

```bash
# Rust 单元测试
cd src-tauri
cargo test

# 集成测试
cargo test --test integration_test

# 后端验证
./test-backend.sh
```

### 编写测试

- 为新功能编写单元测试
- 为关键路径编写集成测试
- 确保测试覆盖率 > 70%

## 发布流程

1. 更新版本号（`package.json` 和 `Cargo.toml`）
2. 更新 `CHANGELOG.md`
3. 运行完整测试套件
4. 构建所有平台的安装包
5. 创建 GitHub Release
6. 发布到包管理器（可选）

## 行为准则

- 尊重所有贡献者
- 保持友好和专业的沟通
- 接受建设性的反馈
- 关注项目目标和用户需求

## 获取帮助

如果你有任何问题，可以：

- 创建 Issue
- 查看现有文档
- 联系维护者

感谢你的贡献！🎉
