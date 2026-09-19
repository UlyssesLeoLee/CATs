//! 合规开关
//!
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §3.3 选项 C
//!   - 合规项目走本地 vLLM + Qwen2.5-7B-Instruct (AWQ 量化版)
//!   - 非合规项目走公有云 OpenAI / Anthropic API
//!   - 通过 LiteLLM Proxy 抽象
//! 引用: doc/05-其他/错误码/CATs_错误码表_v1.0.1.md §3.7 (COMPLIANCE_BLOCKED 409)
//!
//! MVP 行为:
//! - Cloud mode: 允许调云端 provider (OpenAI / Anthropic / Gemini / DeepSeek)
//! - Local mode: 强制只用本地 provider (未实现 → 409 COMPLIANCE_BLOCKED, fail-closed)
//! - 双路径 fallback: 云端故障 → 降级本地 (若项目允许); 本地故障 → 明确错误不静默回云端

use serde::{Deserialize, Serialize};

use crate::error::ProviderError;

/// 合规模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceMode {
    /// 公有云 (OpenAI / Anthropic / Gemini / DeepSeek)
    Cloud,
    /// 本地 (vLLM + Qwen2.5-7B-Instruct AWQ)
    Local,
}

impl ComplianceMode {
    /// 解析字符串 ("cloud" / "local"), 失败 → Cloud 默认 (MVP 容忍缺省)
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            _       => Self::Cloud,
        }
    }

    /// 字符串表示
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
        }
    }
}

/// 合规门: 在 router 之前检查 provider 是否被允许
pub struct ComplianceGate {
    /// 当前合规模式 (per org / per project, MVP 全局)
    mode: ComplianceMode,
}

impl ComplianceGate {
    /// 构造 (默认 Cloud)
    pub fn new(mode: ComplianceMode) -> Self {
        Self { mode }
    }

    /// 获取当前模式
    pub fn mode(&self) -> ComplianceMode { self.mode }

    /// 切换模式 (per project, MVP 简化: 全局)
    pub fn set_mode(&mut self, mode: ComplianceMode) {
        self.mode = mode;
    }

    /// 检查 provider 名是否被当前模式允许
    ///
    /// - Cloud mode: 仅允许 cloud provider (openai/anthropic/gemini/deepseek)
    /// - Local mode: 仅允许 local provider (local)
    ///
    /// 不允许 → COMPLIANCE_BLOCKED 409 (fail-closed, per ADR-010 §4 "本地故障 → 明确错误不静默回云端")
    pub fn check(&self, provider_name: &str) -> Result<(), ProviderError> {
        let is_cloud = matches!(provider_name, "openai" | "anthropic" | "gemini" | "deepseek");
        let is_local = provider_name == "local";

        let allowed = match self.mode {
            ComplianceMode::Cloud => is_cloud,
            ComplianceMode::Local => is_local,
        };

        if allowed {
            Ok(())
        } else {
            Err(ProviderError::ComplianceBlocked {
                message: format!(
                    "provider '{}' is not allowed in compliance_mode='{}'",
                    provider_name, self.mode.as_str()
                ),
                mode: self.mode.as_str().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        assert_eq!(ComplianceMode::parse("cloud"), ComplianceMode::Cloud);
        assert_eq!(ComplianceMode::parse("LOCAL"), ComplianceMode::Local);
        assert_eq!(ComplianceMode::parse(""), ComplianceMode::Cloud); // 默认
        assert_eq!(ComplianceMode::Cloud.as_str(), "cloud");
        assert_eq!(ComplianceMode::Local.as_str(), "local");
    }

    #[test]
    fn cloud_mode_allows_cloud_providers() {
        let gate = ComplianceGate::new(ComplianceMode::Cloud);
        assert!(gate.check("openai").is_ok());
        assert!(gate.check("anthropic").is_ok());
        assert!(gate.check("gemini").is_ok());
        assert!(gate.check("deepseek").is_ok());
    }

    #[test]
    fn cloud_mode_blocks_local_provider() {
        let gate = ComplianceGate::new(ComplianceMode::Cloud);
        let err = gate.check("local").unwrap_err();
        match err {
            ProviderError::ComplianceBlocked { mode, .. } => {
                assert_eq!(mode, "cloud");
            }
            other => panic!("expected ComplianceBlocked, got {:?}", other),
        }
    }

    #[test]
    fn local_mode_blocks_cloud_providers() {
        let gate = ComplianceGate::new(ComplianceMode::Local);
        for p in ["openai", "anthropic", "gemini", "deepseek"] {
            let err = gate.check(p).unwrap_err();
            assert!(matches!(err, ProviderError::ComplianceBlocked { .. }));
        }
    }

    #[test]
    fn local_mode_allows_local_provider() {
        let gate = ComplianceGate::new(ComplianceMode::Local);
        assert!(gate.check("local").is_ok());
    }

    #[test]
    fn set_mode_switches_policy() {
        let mut gate = ComplianceGate::new(ComplianceMode::Cloud);
        assert!(gate.check("openai").is_ok());
        gate.set_mode(ComplianceMode::Local);
        assert!(gate.check("openai").is_err());
        assert!(gate.check("local").is_ok());
    }
}