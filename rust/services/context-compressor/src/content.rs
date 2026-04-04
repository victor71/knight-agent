//! Content type detection for compression rules

use serde::{Deserialize, Serialize};

/// Content type detection for compression rules
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Code,
    Log,
    Text,
    Config,
    System,
}

impl ContentType {
    /// Detect content type from message content
    pub fn detect(content: &str, role: &str) -> Self {
        // System messages are preserved
        if role == "system" {
            return ContentType::System;
        }

        // Check for code patterns
        if content.contains("```") || content.contains("def ") || content.contains("class ")
            || content.contains("function ") || content.contains("pub fn")
        {
            return ContentType::Code;
        }

        // Check for log patterns
        if content.contains("[LOG]") || content.contains("DEBUG") || content.contains("INFO")
            || content.contains(" WARN") || regex::Regex::new(r"\d{4}-\d{2}-\d{2}").map(|r| r.is_match(content)).unwrap_or(false)
        {
            return ContentType::Log;
        }

        // Check for config patterns
        if (content.trim().starts_with('{') && content.trim().ends_with('}'))
            || (content.trim().starts_with('[') && content.trim().ends_with(']'))
        {
            return ContentType::Config;
        }

        ContentType::Text
    }
}

// Minimal regex stub for content detection
mod regex {
    pub struct Regex;
    impl Regex {
        pub fn new(_pattern: &str) -> Result<Self, ()> {
            Ok(Self)
        }
        pub fn is_match(&self, _text: &str) -> bool {
            false
        }
    }
}
