# Aura 测试示例

## 🧪 测试用例

### 测试 1: 中文口语转专业邮件

**输入**:
```
呃，小李啊，那个...明天上海那个会，你帮我准备个 PPT，主讲是苏，别忘了带那个 10% 的折价券...
```

**设置**:
- 格式: Email
- 语气: Professional

**预期输出**:
```
主题：明天上海会议准备事项

小李：

请为明天在上海举行的会议准备 PPT。主讲人是苏总。

请务必携带 10% 折扣券。

谢谢！
```

---

### 测试 2: 随意想法转周报

**输入**:
```
这周做了啥呢，嗯，主要是修了那个登录的 bug，然后呢，还优化了一下数据库查询，对了，还开了几个会讨论新功能，嗯，下周计划继续做性能优化
```

**设置**:
- 格式: Weekly Report
- 语气: Professional

**预期输出**:
```
本周工作总结

主要工作：
1. 修复登录系统 bug
2. 优化数据库查询性能
3. 参与新功能讨论会议

下周计划：
- 继续进行性能优化工作
```

---

### 测试 3: 英文口语转社交媒体文案

**输入**:
```
uh so like today I went to this amazing coffee shop you know and um the latte was really good I mean really really good
```

**设置**:
- 格式: Social Media
- 语气: Casual

**预期输出**:
```
Today I discovered an amazing coffee shop! ☕ The latte was absolutely delicious. Highly recommend checking it out!
```

---

### 测试 4: 技术讨论转代码注释

**输入**:
```
这个函数呢，主要是用来处理用户输入的，嗯，首先会验证输入长度，然后呢会去除多余的空格，最后返回清理后的文本
```

**设置**:
- 格式: Code Comment
- 语气: Professional

**预期输出**:
```
/**
 * Process user input
 * 
 * This function validates input length, removes extra whitespace,
 * and returns the cleaned text.
 * 
 * @param input - Raw user input string
 * @returns Cleaned and validated text
 */
```

---

### 测试 5: 自动格式检测

**输入**:
```
今天天气真好，想去公园走走，顺便拍点照片发朋友圈
```

**设置**:
- 格式: Auto Detect Format
- 语气: Casual

**预期输出** (应自动识别为 Social Media):
```
今天天气超好！准备去公园走走，拍点美照分享给大家 📸✨
```

---

## 🎯 测试步骤

1. 启动应用: `npm run tauri dev`
2. 等待初始化完成（看到 "Initializing..." 消失）
3. 复制测试输入到左侧文本框
4. 选择对应的格式和语气
5. 点击 "Refine Text" 按钮
6. 查看右侧输出结果
7. 验证输出是否符合预期

## 📊 性能指标

预期性能（使用 qwen3.5:2b）:
- 初始化时间: < 5 秒
- 处理延迟: 2-5 秒（取决于输入长度）
- 内存占用: < 2GB
- 置信度: > 80%

## 🐛 已知问题

1. ASR 语音识别尚未完全实现（占位符）
2. 音频录制功能待实现
3. LanceDB 向量搜索为占位符实现
4. 进度反馈机制待完善

## ✅ 验证清单

- [ ] 应用成功启动
- [ ] Ollama 连接正常
- [ ] 文本输入正常
- [ ] 格式选择生效
- [ ] 语气调整生效
- [ ] 输出文本质量良好
- [ ] 置信度显示正常
- [ ] 处理时间合理
- [ ] 复制功能正常

## 🔄 下一步测试

完成当前测试后，下一步将测试：
1. 音频文件上传和转录
2. 用户上下文个性化（名称映射、地点偏好）
3. 纠正历史学习
4. 高级 UI 交互（词句选择、替代选项）
