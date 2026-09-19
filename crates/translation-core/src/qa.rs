//! translation-core QA 模块
//!
//! MVP 实现:
//! - 术语强制: target_text 必须包含所有 expected_terms.target_term
//! - 标签保护: 任何 `{...}` / `<...>` 占位符必须原样保留

use cats_proto::cats::v1::{QAViolation, TermItem};

pub struct QaEngine;

impl QaEngine {
    /// 返回违规列表 (空 = 通过)
    pub fn run(
        segment_id: &str,
        source_text: &str,
        target_text: &str,
        expected_terms: &[TermItem],
    ) -> Vec<QAViolation> {
        let mut violations = vec![];

        // 1. 术语强制 (expected_terms 必含)
        for term in expected_terms {
            if term.forbidden && target_text.contains(&term.target_term) {
                violations.push(QAViolation {
                    rule_id: "glossary.forbidden".into(),
                    rule_category: "glossary".into(),
                    message: format!("forbidden term appears: {}", term.target_term),
                    start_pos: 0,
                    end_pos: term.target_term.len() as i32,
                    severity: "ERROR".into(),
                    ..Default::default()
                });
            }
            if !term.forbidden && !term.target_term.is_empty() && !target_text.contains(&term.target_term) {
                violations.push(QAViolation {
                    rule_id: "glossary.missing".into(),
                    rule_category: "glossary".into(),
                    message: format!("expected term missing: {}", term.target_term),
                    start_pos: 0,
                    end_pos: 0,
                    severity: "WARNING".into(),
                    ..Default::default()
                });
            }
        }

        // 2. 标签保护 - 占位符完整性
        for open_char in ['{', '<'] {
            let close_char = match open_char {
                '{' => '}',
                '<' => '>',
                _ => continue,
            };
            let opens = source_text.matches(open_char).count();
            let closes = target_text.matches(close_char).count();
            if opens != closes {
                violations.push(QAViolation {
                    rule_id: format!("placeholder.{}", open_char),
                    rule_category: "placeholder".into(),
                    message: format!(
                        "placeholder count mismatch in segment {}",
                        segment_id
                    ),
                    start_pos: 0,
                    end_pos: 0,
                    severity: "ERROR".into(),
                    ..Default::default()
                });
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_terms_passes() {
        let v = QaEngine::run("s1", "Hello", "你好", &[]);
        assert!(v.is_empty());
    }

    #[test]
    fn forbidden_term_fails() {
        let terms = vec![TermItem {
            source_term: "secret".into(),
            target_term: "秘密".into(),
            forbidden: true,
            ..Default::default()
        }];
        let v = QaEngine::run("s1", "secret data", "秘密数据", &terms);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "glossary.forbidden");
    }

    #[test]
    fn placeholder_count_mismatch_fails() {
        let v = QaEngine::run("s1", "Hello {name}", "你好", &[]);
        assert!(v.iter().any(|x| x.rule_id == "placeholder.{"));
    }
}