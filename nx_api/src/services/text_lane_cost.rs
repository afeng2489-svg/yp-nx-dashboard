//! AF-MM-04：文本车道成本路由（规则表，非 LLM）

/// 根据阶段名与成本偏好解析 API 模型 ID
pub fn resolve_text_lane_model(base_model: &str, stage_name: Option<&str>, cost_mode: Option<&str>) -> String {
    let mode = cost_mode.unwrap_or("quality");
    if mode != "cost" {
        return base_model.to_string();
    }

    let stage = stage_name.unwrap_or("").to_lowercase();
    if stage.contains("摘要") || stage.contains("summary") {
        return pick_cheaper(base_model, &["gpt-4o-mini", "deepseek-chat", "claude-haiku-4-5"]);
    }
    if stage.contains("计划") || stage.contains("plan") || stage.contains("规划") {
        return pick_cheaper(base_model, &["gpt-4o", "deepseek-chat", "claude-sonnet-4-5"]);
    }
    if stage.contains("审查") || stage.contains("review") {
        return pick_cheaper(base_model, &["gpt-4o-mini", "deepseek-chat", "claude-haiku-4-5"]);
    }
    pick_cheaper(base_model, &["gpt-4o-mini", "deepseek-chat", "claude-haiku-4-5"])
}

fn pick_cheaper(base: &str, candidates: &[&str]) -> String {
    let base_lower = base.to_lowercase();
    for c in candidates {
        if base_lower.contains(&c.to_lowercase()) {
            return base.to_string();
        }
    }
    candidates.first().unwrap_or(&base).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_mode_downgrades_summary() {
        let m = resolve_text_lane_model("claude-sonnet-4-5", Some("交付摘要"), Some("cost"));
        assert!(m.contains("mini") || m.contains("haiku") || m.contains("deepseek"));
    }

    #[test]
    fn quality_mode_keeps_base() {
        assert_eq!(
            resolve_text_lane_model("claude-sonnet-4-5", Some("交付摘要"), Some("quality")),
            "claude-sonnet-4-5"
        );
    }
}
