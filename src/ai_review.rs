use std::fs::read_to_string;
use std::path::Path;
use std::time::Duration;

use crate::config::Config;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tr::tr;

const DEFAULT_TIMEOUT: u64 = 30;
const DEFAULT_MAX_TOKENS: usize = 2048;

#[derive(Debug, Clone)]
pub struct ReviewInput<'a> {
    pub pkg: &'a str,
    pub diff: &'a str,
    pub pkgbuild_path: Option<&'a Path>,
}

#[derive(Debug, Clone)]
pub struct ReviewOutput {
    pub risk: RiskLevel,
    pub summary: String,
    pub concerns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" => Some(RiskLevel::Low),
            "medium" => Some(RiskLevel::Medium),
            "med" => Some(RiskLevel::Medium),
            "high" => Some(RiskLevel::High),
            _ => None,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            RiskLevel::Low => "✓",
            RiskLevel::Medium => "!",
            RiskLevel::High => "✗",
        }
    }
}

#[derive(Serialize, Debug)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize, Debug)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Serialize, Debug)]
struct ResponseFormat {
    r#type: &'static str,
}

#[derive(Deserialize, Debug)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize, Debug, Default)]
struct ParsedReview {
    #[serde(default)]
    risk: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    concerns: Vec<String>,
}

pub async fn review(config: &Config, input: &ReviewInput<'_>) -> Result<ReviewOutput> {
    let url = config
        .ai_review_url
        .as_deref()
        .map(str::trim)
        .map(|s| s.trim_matches(&['"', '\'', ' '] as &[_]))
        .context(tr!("AI review enabled but no URL configured"))?;
    let model = config
        .ai_review_model
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let timeout = Duration::from_secs(if config.ai_review_timeout > 0 {
        config.ai_review_timeout
    } else {
        DEFAULT_TIMEOUT
    });
    let max_tokens = if config.ai_review_max_tokens > 0 {
        config.ai_review_max_tokens
    } else {
        DEFAULT_MAX_TOKENS
    };

    let system_prompt = "You are a security reviewer for Arch Linux AUR packages. \
        Review the provided PKGBUILD diff and full file for signs of malware, \
        supply-chain attacks, or any other suspicious behaviour. \
        Be concise. Return your assessment as JSON with keys: \
        risk (low|medium|high), summary (one sentence), concerns (list of specific issues). \
        If nothing looks suspicious, return risk: low, an empty concerns list, and a brief summary.";

    let user_prompt = build_prompt(input);

    let request = ChatCompletionRequest {
        model,
        messages: vec![
            Message {
                role: "system",
                content: system_prompt.to_string(),
            },
            Message {
                role: "user",
                content: user_prompt,
            },
        ],
        max_tokens,
        response_format: Some(ResponseFormat { r#type: "json_object" }),
    };

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context(tr!("failed to build AI review HTTP client"))?;

    let mut builder = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .json(&request);

    if let Some(key) = &config.ai_review_key {
        builder = builder.header(AUTHORIZATION, format!("Bearer {key}"));
    }

    let response = builder.send().await.map_err(|e| {
        anyhow!(tr!("failed to contact AI review endpoint — {e}", e = e))
    })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!(tr!(
            "AI review endpoint returned {status}: {body}",
            status = status,
            body = body
        ));
    }

    let completion: ChatCompletionResponse = response
        .json()
        .await
        .context(tr!("failed to parse AI review response"))?;

    let content = completion
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .unwrap_or_default();

    parse_review_response(&content)
}

fn build_prompt(input: &ReviewInput<'_>) -> String {
    let mut prompt = format!("Package: {}\n\n", input.pkg);

    prompt.push_str("Diff since last installed version:\n");
    prompt.push_str("```diff\n");
    prompt.push_str(input.diff);
    prompt.push_str("\n```\n\n");

    if let Some(path) = input.pkgbuild_path {
        match read_to_string(path) {
            Ok(pkgbuild) => {
                prompt.push_str("Full PKGBUILD:\n");
                prompt.push_str("```bash\n");
                prompt.push_str(&pkgbuild);
                prompt.push_str("\n```\n\n");
            }
            Err(_) => {
                prompt.push_str("Full PKGBUILD: (unreadable)\n\n");
            }
        }
    }

    prompt
}

fn parse_review_response(content: &str) -> Result<ReviewOutput> {
    // Some providers wrap JSON in markdown fences.
    let cleaned = content
        .trim()
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| content.strip_prefix("```").and_then(|s| s.strip_suffix("```")))
        .unwrap_or(content)
        .trim();

    let parsed: ParsedReview = serde_json::from_str(cleaned)
        .with_context(|| tr!("AI review returned invalid JSON: {content}", content = content))?;

    let risk = RiskLevel::from_str(&parsed.risk).unwrap_or(RiskLevel::Low);
    let summary = if parsed.summary.is_empty() {
        tr!("no summary provided")
    } else {
        parsed.summary
    };

    Ok(ReviewOutput {
        risk,
        summary,
        concerns: parsed.concerns,
    })
}

/// Wraps `text` to `max_width` columns, breaking on word boundaries.
/// Continuation lines are indented with `hang` spaces.
pub fn wrap_text(text: &str, max_width: usize, hang: &str) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    let mut line_len: usize = 0;
    let mut first = true;

    for word in text.split_whitespace() {
        let wlen = word.chars().count();
        let needed = if first { 0 } else { 1 } + wlen;
        if !first && line_len + needed > max_width {
            out.push('\n');
            out.push_str(hang);
            out.push_str(word);
            line_len = hang.chars().count().saturating_add(wlen);
        } else {
            if !first {
                out.push(' ');
                line_len += 1;
            }
            out.push_str(word);
            line_len = line_len.saturating_add(wlen);
        }
        first = false;
    }

    out
}

pub fn print_review(config: &Config, pkg: &str, review: &ReviewOutput) {
    let c = &config.color;
    let risk_color = match review.risk {
        RiskLevel::Low => c.upgrade,
        RiskLevel::Medium => c.warning,
        RiskLevel::High => c.error,
    };

    let header_prefix = format!(
        "{} {}: {} {} - ",
        tr!("AI review"),
        pkg,
        review.risk.icon(),
        review.risk.as_str()
    );

    let wrapped = wrap_text(&review.summary, 80, "    ");
    let summary_first_line = wrapped.lines().next().unwrap_or("");
    let summary_rest: Vec<&str> = wrapped.lines().skip(1).collect();

    println!(
        "{} {} {}",
        c.action.paint("::"),
        c.bold.paint(&header_prefix),
        risk_color.paint(summary_first_line)
    );

    for line in summary_rest {
        println!("{}", risk_color.paint(line));
    }

    for concern in &review.concerns {
        let wrapped = wrap_text(concern, 74, "      "); // 80 - "    - "
        let mut lines = wrapped.lines();
        if let Some(first) = lines.next() {
            println!("    {} {}", c.warning.paint("-"), first);
        }
        for line in lines {
            println!("      {}", line);
        }
    }
}
