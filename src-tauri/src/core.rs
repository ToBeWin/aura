use crate::errors::{AuraError, Result};
use crate::llm::{LocalLLM, PromptTemplates};
use crate::models::{AppliedRule, CorrectionRecord, LLMProviderSettings, RefinedOutput};
use crate::processing::DeNoisingModule;
use crate::storage::{LocalVectorDB, UserContextStore};
use crate::text::normalize_to_simplified_chinese;
use regex::Regex;
use std::time::Instant;

pub struct AuraCore {
    pub llm: LocalLLM,
    denoise_module: DeNoisingModule,
    pub context_store: UserContextStore,
    correction_db: LocalVectorDB,
}

/// Simple result from the typeless refine pipeline
#[derive(Debug)]
pub struct SimpleRefine {
    pub text: String,
    pub confidence: f64,
    pub applied_rules: Vec<AppliedRule>,
    pub output_mode: String,
}

impl AuraCore {
    fn detect_language_hint(text: &str) -> &'static str {
        let mut ascii_letters = 0usize;
        let mut cjk_chars = 0usize;

        for ch in text.chars() {
            if ch.is_ascii_alphabetic() {
                ascii_letters += 1;
            } else if ('\u{4E00}'..='\u{9FFF}').contains(&ch) {
                cjk_chars += 1;
            }
        }

        if ascii_letters >= 8 && ascii_letters > cjk_chars * 2 {
            "en"
        } else if cjk_chars >= 2 {
            "zh"
        } else {
            "auto"
        }
    }

    fn strip_refine_prefixes(text: &str) -> String {
        let mut value = text.trim().trim_matches('`').trim().to_string();

        if let Ok(re) = Regex::new(
            r"(?im)^(?:精炼后的文本|精炼后|润色后|最终成稿|输出结果|输出|结果|Refined text|Refined version|Final text|Final answer)\s*[:：]\s*",
        ) {
            value = re.replace_all(&value, "").to_string();
        }

        if let Ok(re) = Regex::new(r"(?s)^```(?:text|markdown)?\s*(.*?)\s*```$") {
            if let Some(caps) = re.captures(&value) {
                if let Some(inner) = caps.get(1) {
                    value = inner.as_str().trim().to_string();
                }
            }
        }

        value
    }

    fn normalize_paragraph_breaks(text: &str) -> String {
        let mut value = text.trim().to_string();

        if let Ok(re) = Regex::new(r"\n{3,}") {
            value = re.replace_all(&value, "\n\n").to_string();
        }

        if !value.contains('\n') && value.matches('。').count() >= 2 {
            value = value.replace("。", "。\n");
        }

        if let Ok(re) = Regex::new(r"\n{2,}") {
            value = re.replace_all(&value, "\n").to_string();
        }

        value.trim().to_string()
    }

    fn chinese_number_to_index(token: &str) -> Option<usize> {
        match token {
            "一" => Some(1),
            "二" => Some(2),
            "三" => Some(3),
            "四" => Some(4),
            "五" => Some(5),
            "六" => Some(6),
            "七" => Some(7),
            "八" => Some(8),
            "九" => Some(9),
            "十" => Some(10),
            _ => None,
        }
    }

    fn english_marker_to_index(token: &str) -> Option<usize> {
        match token.to_ascii_lowercase().as_str() {
            "first" | "firstly" => Some(1),
            "second" | "secondly" => Some(2),
            "third" | "thirdly" => Some(3),
            "fourth" | "fourthly" => Some(4),
            "fifth" | "fifthly" => Some(5),
            "sixth" => Some(6),
            "seventh" => Some(7),
            "eighth" => Some(8),
            "ninth" => Some(9),
            "tenth" => Some(10),
            "finally" | "lastly" => Some(99),
            _ => None,
        }
    }

    fn structure_enumerated_notes(text: &str) -> String {
        let marker_re = match Regex::new(r"第([一二三四五六七八九十])(?:点)?") {
            Ok(re) => re,
            Err(_) => return text.to_string(),
        };

        if !marker_re.is_match(text) {
            return text.to_string();
        }

        let mut sections: Vec<(usize, usize, usize)> = Vec::new();
        for caps in marker_re.captures_iter(text) {
            let Some(full) = caps.get(0) else { continue };
            let Some(num) = caps.get(1) else { continue };
            let Some(index) = Self::chinese_number_to_index(num.as_str()) else { continue };
            if sections.last().is_some_and(|last| last.1 == full.start()) {
                continue;
            }
            sections.push((index, full.start(), full.end()));
        }

        if sections.is_empty() {
            return text.to_string();
        }

        let mut lines = Vec::new();
        for (i, (index, _start, end)) in sections.iter().enumerate() {
            let next_start = sections.get(i + 1).map(|(_, start, _)| *start).unwrap_or(text.len());
            let mut content = text[*end..next_start].trim().to_string();
            content = content
                .trim_matches(|c: char| matches!(c, '，' | ',' | '。' | '；' | ';' | '：' | ':'))
                .trim()
                .to_string();

            if content.is_empty() {
                continue;
            }

            if let Ok(re) = Regex::new(r"\s+") {
                content = re.replace_all(&content, " ").to_string();
            }

            if !content.ends_with(['。', '！', '？']) {
                content.push('。');
            }

            lines.push(format!("{}. {}", index, content));
        }

        if lines.is_empty() {
            return text.to_string();
        }

        lines.join("\n")
    }

    fn structure_english_enumerated_notes(text: &str) -> String {
        let marker_re = match Regex::new(
            r"(?i)\b(first|firstly|second|secondly|third|thirdly|fourth|fourthly|fifth|fifthly|sixth|seventh|eighth|ninth|tenth|finally|lastly)\b[\s,:-]*",
        ) {
            Ok(re) => re,
            Err(_) => return text.to_string(),
        };

        if !marker_re.is_match(text) {
            return text.to_string();
        }

        let mut sections: Vec<(usize, usize, usize)> = Vec::new();
        for caps in marker_re.captures_iter(text) {
            let Some(full) = caps.get(0) else { continue };
            let Some(token) = caps.get(1) else { continue };
            let Some(index) = Self::english_marker_to_index(token.as_str()) else { continue };
            if sections.last().is_some_and(|last| last.1 == full.start()) {
                continue;
            }
            sections.push((index, full.start(), full.end()));
        }

        if sections.len() < 2 {
            return text.to_string();
        }

        let mut lines = Vec::new();
        let mut display_index = 1usize;

        for (i, (index, _start, end)) in sections.iter().enumerate() {
            let next_start = sections.get(i + 1).map(|(_, start, _)| *start).unwrap_or(text.len());
            let mut content = text[*end..next_start].trim().to_string();
            content = content
                .trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':' | '-' | ' '))
                .trim()
                .to_string();

            if content.is_empty() {
                continue;
            }

            if let Ok(re) = Regex::new(r"\s+") {
                content = re.replace_all(&content, " ").to_string();
            }

            let number = if *index == 99 { display_index } else { *index };
            if !content.ends_with(['.', '!', '?']) {
                content.push('.');
            }
            lines.push(format!("{}. {}", number, content));
            display_index = number + 1;
        }

        if lines.is_empty() {
            return text.to_string();
        }

        lines.join("\n")
    }

    fn finalize_refined_text(text: &str) -> String {
        let stripped = Self::strip_refine_prefixes(text);
        let simplified = normalize_to_simplified_chinese(stripped.trim());
        let structured_cn = Self::structure_enumerated_notes(&simplified);
        let structured = Self::structure_english_enumerated_notes(&structured_cn);
        Self::normalize_paragraph_breaks(&structured)
    }

    fn polish_fallback_text(text: &str) -> String {
        let mut value = text.trim().to_string();
        if value.is_empty() {
            return value;
        }

        if let Ok(re) = Regex::new(r"[？?]{2,}") {
            value = re.replace_all(&value, "？").to_string();
        }
        if let Ok(re) = Regex::new(r"[！!]{2,}") {
            value = re.replace_all(&value, "！").to_string();
        }
        if let Ok(re) = Regex::new(r"[。\.]{2,}") {
            value = re.replace_all(&value, "。").to_string();
        }
        if let Ok(re) = Regex::new(r"[，,]{2,}") {
            value = re.replace_all(&value, "，").to_string();
        }
        if let Ok(re) = Regex::new(r"\s+") {
            value = re.replace_all(&value, " ").to_string();
        }

        let sentence_re = Regex::new(r"[。！？!?]+").ok();
        let mut unique_sentences: Vec<String> = Vec::new();
        if let Some(re) = sentence_re {
            for sentence in re.split(&value) {
                let trimmed = sentence.trim().trim_matches('，').trim();
                if trimmed.is_empty() {
                    continue;
                }
                if unique_sentences.last().is_some_and(|last| last == trimmed) {
                    continue;
                }
                unique_sentences.push(trimmed.to_string());
            }
        }

        if !unique_sentences.is_empty() {
            value = unique_sentences.join("。");
        }

        if !value.ends_with(['。', '！', '？']) {
            value.push('。');
        }

        Self::finalize_refined_text(&value)
    }

    pub fn new(
        model_name: String,
        db_path: String,
        vector_db_path: String,
    ) -> Result<Self> {
        Self::new_with_settings(
            &LLMProviderSettings {
                local_model: model_name,
                ..LLMProviderSettings::default()
            },
            db_path,
            vector_db_path,
        )
    }

    pub fn new_with_settings(
        llm_settings: &LLMProviderSettings,
        db_path: String,
        vector_db_path: String,
    ) -> Result<Self> {
        let llm = LocalLLM::from_settings(llm_settings);

        // Spawn background task to preload model
        let llm_clone = llm.clone();
        tokio::spawn(async move {
            if let Err(e) = llm_clone.preload().await {
                log::warn!("Failed to preload model: {:?}", e);
            }
        });

        let denoise_module = DeNoisingModule::new(llm.clone());
        let context_store = UserContextStore::new(&db_path)?;
        let correction_db = LocalVectorDB::new(vector_db_path)?;

        Ok(Self {
            llm,
            denoise_module,
            context_store,
            correction_db,
        })
    }

    #[allow(dead_code)]
    pub async fn refine_thought(
        &self,
        raw_input: &str,
        user_id: &str,
        output_format: Option<&str>,
        tone: &str,
    ) -> Result<RefinedOutput> {
        let refined = self.refine_simple(raw_input, user_id, output_format).await?;
        let start = Instant::now();

        Ok(RefinedOutput {
            text: refined.text,
            format: refined.output_mode,
            tone: tone.to_string(),
            confidence: refined.confidence,
            processing_time: start.elapsed().as_secs_f64(),
            applied_rules: refined.applied_rules,
            metadata: Default::default(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Typeless pipeline: raw_input → denoise → LLM full refinement at the current cursor position
    pub async fn refine_simple(
        &self,
        raw_input: &str,
        user_id: &str,
        output_mode: Option<&str>,
    ) -> Result<SimpleRefine> {
        let start = Instant::now();
        let selected_mode = output_mode.unwrap_or("note");

        // Input validation
        if raw_input.is_empty() || raw_input.len() > 10000 {
            return Err(AuraError::InputValidation {
                message: "Input is empty or exceeds 10000 characters".to_string(),
                error_code: "INPUT_001".to_string(),
            });
        }

        // Stage 1: Denoise (remove filler words "嗯", "啊", "那个", etc.)
        let denoise_result = self.denoise_module.denoise(raw_input).await?;
        log::info!(
            "[Aura] Denoise confidence={:.2} cleaned={}",
            denoise_result.confidence,
            denoise_result.cleaned_text
        );

        // Apply user context substitutions (name mappings, terminology, forbidden words)
        let mut final_text = denoise_result.cleaned_text.clone();
        let mut applied_rules: Vec<AppliedRule> = Vec::new();

        if let Ok(ctx) = self.context_store.get_context(user_id) {
            // Name mappings
            for (short, full) in &ctx.name_mappings {
                if final_text.contains(short) {
                    final_text = final_text.replace(short, full);
                    applied_rules.push(AppliedRule {
                        rule_type: "name_mapping".to_string(),
                        from: short.clone(),
                        to: full.clone(),
                    });
                }
            }

            // Terminology
            for (term, preferred) in &ctx.terminology {
                if final_text.contains(term) {
                    final_text = final_text.replace(term, preferred);
                    applied_rules.push(AppliedRule {
                        rule_type: "terminology".to_string(),
                        from: term.clone(),
                        to: preferred.clone(),
                    });
                }
            }
        }

        let retrieved_corrections = match self.retrieve_similar_corrections(user_id, &final_text).await {
            Ok(results) => results,
            Err(error) => {
                log::warn!("Silent correction retrieval unavailable, skipping: {:?}", error);
                Vec::new()
            }
        };
        let (corrected_text, correction_rules, correction_guidance) =
            self.apply_silent_corrections(&final_text, &retrieved_corrections);
        final_text = corrected_text;
        applied_rules.extend(correction_rules);
        log::info!(
            "[Aura] Personalization corrections={} guidance_lines={}",
            retrieved_corrections.len(),
            correction_guidance.lines().count()
        );

        let source_language = Self::detect_language_hint(&final_text);
        log::info!("[Aura] Refine source language={}", source_language);

        // Stage 2: LLM full refinement — reorganize into fluent, polished prose
        let refine_prompt = if selected_mode == "note" && correction_guidance.is_empty() {
            PromptTemplates::refine(&final_text)
        } else if selected_mode == "note" {
            PromptTemplates::refine_with_guidance(&final_text, &correction_guidance)
        } else {
            PromptTemplates::refine_for_mode(&final_text, &correction_guidance, selected_mode)
        };
        let trimmed = match self
            .llm
            .generate(
                &refine_prompt,
                Some(&PromptTemplates::refine_system(source_language)),
                Some(1024),
                0.25,
            )
            .await
        {
            Ok(output) => Self::finalize_refined_text(output.trim()),
            Err(error) => {
                log::warn!("LLM refine unavailable, fallback to cleaned text: {:?}", error);
                let fallback_text = Self::polish_fallback_text(&final_text);
                applied_rules.push(AppliedRule {
                    rule_type: "llm_fallback".to_string(),
                    from: "llm_unavailable".to_string(),
                    to: fallback_text.clone(),
                });
                fallback_text
            }
        };

        let elapsed = start.elapsed().as_secs_f64();
        log::info!(
            "Refined {} -> {} in {:.3}s",
            raw_input.len(),
            trimmed.len(),
            elapsed
        );

        if applied_rules.iter().all(|rule| rule.rule_type != "llm_fallback") {
            applied_rules.push(AppliedRule {
                rule_type: "llm_refine".to_string(),
                from: raw_input.chars().take(30).collect(),
                to: trimmed.chars().take(30).collect(),
            });
        }

        Ok(SimpleRefine {
            text: trimmed,
            confidence: 0.85,
            applied_rules,
            output_mode: selected_mode.to_string(),
        })
    }

    async fn retrieve_similar_corrections(
        &self,
        user_id: &str,
        text: &str,
    ) -> Result<Vec<CorrectionRecord>> {
        let _ = (user_id, text);
        // Correction-memory retrieval is disabled for this release.
        Ok(Vec::new())
    }

    fn apply_silent_corrections(
        &self,
        text: &str,
        corrections: &[CorrectionRecord],
    ) -> (String, Vec<AppliedRule>, String) {
        let mut result = text.to_string();
        let mut applied_rules = Vec::new();
        let mut guidance_lines = Vec::new();

        for correction in corrections {
            let pattern = correction.pattern.trim();
            let replacement = correction.replacement.trim();

            if pattern.is_empty() || replacement.is_empty() || pattern == replacement {
                continue;
            }

            if pattern.len() <= 80 && result.contains(pattern) {
                result = result.replace(pattern, replacement);
                applied_rules.push(AppliedRule {
                    rule_type: "silent_correction".to_string(),
                    from: pattern.to_string(),
                    to: replacement.to_string(),
                });
            }

            guidance_lines.push(format!(
                "- Prefer `{}` over `{}` when the meaning matches. Correction type: {}.",
                replacement,
                pattern,
                correction.correction_type
            ));
        }

        (result, applied_rules, guidance_lines.join("\n"))
    }
}
