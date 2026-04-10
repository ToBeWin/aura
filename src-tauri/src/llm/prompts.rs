pub struct PromptTemplates;

impl PromptTemplates {
    pub fn refine_system(source_language: &str) -> String {
        let language_rule = match source_language {
            "en" => "The source language is English. The output must remain English. Do not translate it into Chinese.",
            "zh" => "源语言是中文。输出必须保持为中文，且中文输出必须使用简体中文。绝对不要翻译成英文。",
            _ => "The output language must stay the same as the source language. Never translate unless the user explicitly asks for translation.",
        };

        format!(
            r#"你是 Aura 的成稿润色引擎。

你的职责不是逐字转写，而是将用户的自然口语重写为可以直接发送、提交、粘贴使用的正式文本。

必须严格遵守以下规则：
1. 保留原意，不编造事实，不补充用户没说过的信息。
2. 删除口语赘词、重复表达、犹豫词和无意义语气词。
3. 把碎片化表达整理成完整句子，提升逻辑、顺序和可读性。
4. 如果原文有明显层级信号，例如“第一、第二、第三”或“first, second, third, finally”，必须整理成清晰的编号列表，每一点单独成行。
5. 如果原文本质上是一段说明、请求、总结或观点表达，请输出像最终成稿一样的正文，而不是聊天记录或逐句誊写。
6. 中文输出必须使用简体中文。
7. 除非用户原文明确要求，否则不要添加标题、称呼、落款、解释、备注、引号或项目符号说明。
8. 只输出最终正文本身，不要输出“润色后”“修改后”“Here is the refined version”之类前缀。
9. {}
"#,
            language_rule
        )
    }

    fn mode_instructions(output_mode: &str) -> &'static str {
        match output_mode {
            "email" => "输出为简洁、专业、可直接发送的邮件正文。",
            "report" => "输出为结构清晰的简短汇报，可用于日报、周报或进度同步。",
            "social" => "输出为适合社交平台发布的短文案，保持自然、清晰、有节奏。",
            _ => "输出为自然、清晰、可直接使用的笔记正文。",
        }
    }

    pub fn denoise(raw_input: &str) -> String {
        format!(
            r#"你是一个文本去噪专家。请移除以下口语文本中的填充词（如"呃"、"那个"、"嗯"、"啊"、"就是"、"然后"、"嘛"、"uh"、"um"、"like"）和冗余表达，保持原始语义不变。

原始文本：
{}

去噪后的文本："#,
            raw_input
        )
    }

    pub fn refine(raw_input: &str) -> String {
        format!(
            r#"请将下面这段口语内容直接改写为可使用的成稿正文。

输出要求：
- 更像最终稿，而不是逐字转写
- 如果有层级信号，整理成编号列表
- 不要解释，不要加标题，不要加前缀

原始文本：
{}"#,
            raw_input
        )
    }

    pub fn refine_with_guidance(raw_input: &str, guidance: &str) -> String {
        format!(
            r#"请将下面这段口语内容直接改写为可使用的成稿正文，并优先遵守这些长期偏好与历史纠正规则：

{}

输出要求：
- 更像最终稿，而不是逐字转写
- 如果有层级信号，整理成编号列表
- 不要解释，不要加标题，不要加前缀

原始文本：
{}"#,
            guidance, raw_input
        )
    }

    pub fn refine_for_mode(raw_input: &str, guidance: &str, output_mode: &str) -> String {
        let mode_instruction = Self::mode_instructions(output_mode);
        format!(
            r#"请将下面这段口语内容改写为最终成稿正文。

输出风格要求：
- {}
- 优先遵守这些长期偏好与历史纠正规则：
{}
- 如果有层级信号，整理成编号列表
- 不要解释，不要加标题，不要加前缀

原始文本：
{}"#,
            mode_instruction, guidance, raw_input
        )
    }
}
