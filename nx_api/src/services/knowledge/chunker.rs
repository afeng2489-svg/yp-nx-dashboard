//! 文本分块器
//!
//! 将文档内容按段落 + token 数拆分为 chunks。

/// 文本分块结果
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub index: usize,
    pub content: String,
    pub token_count: usize,
}

/// 将文本拆分为 chunks
///
/// 策略：按段落拆分 → 短段落合并 → 长段落二次拆分
pub fn chunk_text(content: &str, max_tokens: usize) -> Vec<TextChunk> {
    let paragraphs = split_paragraphs(content);
    let merged = merge_short_paragraphs(paragraphs, max_tokens);
    let final_chunks = split_long_paragraphs(merged, max_tokens);
    let mut result = Vec::new();
    for (i, chunk_content) in final_chunks.into_iter().enumerate() {
        let tc = estimate_tokens(&chunk_content);
        result.push(TextChunk {
            index: i,
            content: chunk_content,
            token_count: tc,
        });
    }
    result
}

/// 按空行拆分段落
fn split_paragraphs(content: &str) -> Vec<String> {
    content
        .split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 合并过短的段落（< 100 token 的与下一段合并）
fn merge_short_paragraphs(paragraphs: Vec<String>, max_tokens: usize) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    let mut buffer = String::new();

    for para in paragraphs {
        let buffer_tokens = estimate_tokens(&buffer);
        let para_tokens = estimate_tokens(&para);

        if buffer.is_empty() {
            buffer = para;
            continue;
        }

        // 如果当前段落很短，或者合并后仍不超过 max，则合并
        if para_tokens < 100 || buffer_tokens + para_tokens + 1 <= max_tokens {
            buffer = format!("{}\n\n{}", buffer, para);
        } else {
            merged.push(std::mem::take(&mut buffer));
            buffer = para;
        }
    }

    if !buffer.is_empty() {
        merged.push(buffer);
    }

    merged
}

/// 对过长的段落按句子二次拆分
fn split_long_paragraphs(paragraphs: Vec<String>, max_tokens: usize) -> Vec<String> {
    let mut result = Vec::new();

    for para in paragraphs {
        let tokens = estimate_tokens(&para);
        if tokens <= max_tokens {
            result.push(para);
            continue;
        }

        // 按句子拆分
        let sentences = split_sentences(&para);
        let mut buffer = String::new();

        for sentence in sentences {
            let buffer_tokens = estimate_tokens(&buffer);
            let sentence_tokens = estimate_tokens(&sentence);

            if buffer.is_empty() {
                buffer = sentence;
                continue;
            }

            if buffer_tokens + sentence_tokens + 1 <= max_tokens {
                buffer = format!("{} {}", buffer, sentence);
            } else {
                result.push(std::mem::take(&mut buffer));
                buffer = sentence;
            }
        }

        if !buffer.is_empty() {
            result.push(buffer);
        }
    }

    result
}

/// 按句子边界拆分
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if ch == '。' || ch == '！' || ch == '？' || ch == '.' || ch == '!' || ch == '?' {
            // 检查后面是否有空格或结尾（简单启发式）
            sentences.push(current.trim().to_string());
            current = String::new();
        }
    }

    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }

    sentences
}

/// 简易 token 估算
///
/// 英文约 4 字符/token，中文约 2 字符/token
/// 使用混合估算：统计 CJK 字符和 ASCII 字符分别估算
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut ascii = 0usize;

    for ch in text.chars() {
        if is_cjk(ch) {
            cjk += 1;
        } else {
            ascii += 1;
        }
    }

    // CJK: ~2 字符/token, ASCII: ~4 字符/token
    (cjk + 1) / 2 + (ascii + 3) / 4
}

/// 判断是否为 CJK 字符
fn is_cjk(ch: char) -> bool {
    let cp = ch as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
        // CJK Extension A
        || (0x3400..=0x4DBF).contains(&cp)
        // Katakana + Hiragana
        || (0x3040..=0x30FF).contains(&cp)
        // Hangul
        || (0xAC00..=0xD7AF).contains(&cp)
        // Fullwidth forms
        || (0xFF00..=0xFFEF).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_basic() {
        let content = "First paragraph with some content.\n\nSecond paragraph with more text.\n\nThird short one.";
        let chunks = chunk_text(content, 500);
        assert!(!chunks.is_empty());
        // 短段落可能被合并
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_chunk_text_long_paragraph() {
        let long_para = "This is a sentence. ".repeat(100);
        let content = long_para.as_str();
        let chunks = chunk_text(content, 500);
        assert!(
            chunks.len() > 1,
            "Long text should be split into multiple chunks"
        );
    }

    #[test]
    fn test_estimate_tokens_english() {
        let text = "Hello world this is a test";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0 && tokens < text.len());
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let text = "你好世界这是一个测试";
        let tokens = estimate_tokens(text);
        // 9 CJK chars → ~5 tokens
        assert!(tokens > 0 && tokens <= 9);
    }

    #[test]
    fn test_empty_content() {
        let chunks = chunk_text("", 500);
        assert!(chunks.is_empty());
    }
}
