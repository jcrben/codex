//! Local retirement policy shared by model discovery and inference boundaries.

use crate::error::CodexErr;
use crate::error::Result;

/// Recognizes the retired GPT-6 Sol family without rejecting GPT-6.1 Sol or Luna.
/// Discovery aliases, bracket hints, and repeated `-fast` suffixes do not grant
/// permission to use a retired model. Unrelated and unknown model names retain
/// their existing compatibility behavior.
pub fn retired_codex_sol_model(model: &str) -> bool {
    let normalized = model.trim().to_ascii_lowercase();
    let base = normalized
        .strip_prefix("claude-codex-")
        .unwrap_or(&normalized)
        .split('[')
        .next()
        .unwrap_or_default()
        .trim_end_matches("-fast");
    base == "gpt-6-sol"
}

/// Rejects a retired inference selection locally, without silently substituting
/// another model or consulting provider/auth state.
pub fn ensure_model_not_retired(model: &str) -> Result<()> {
    if retired_codex_sol_model(model) {
        return Err(CodexErr::InvalidRequest(
            "gpt-6-sol is retired. Use gpt-6.1-sol for harder work or gpt-6-luna for routine work."
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "model_policy_tests.rs"]
mod tests;
