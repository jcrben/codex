use super::ensure_model_not_retired;
use super::retired_codex_sol_model;
use pretty_assertions::assert_eq;

#[test]
fn retired_sol_aliases_hints_case_and_fast_suffixes_are_rejected() {
    for model in [
        "gpt-6-sol",
        "gpt-6-sol-fast",
        "gpt-6-sol-fast-fast",
        "gpt-6-sol[1m]",
        "gpt-6-sol-fast[1m]",
        "claude-codex-gpt-6-sol",
        "claude-codex-gpt-6-sol-fast[1m]",
        "  CLAUDE-CODEX-GPT-6-SOL-FAST-FAST[1M]\t",
    ] {
        assert_eq!(retired_codex_sol_model(model), true, "{model}");
        let error = ensure_model_not_retired(model).expect_err("retired model must refuse");
        assert_eq!(error.retry_delay(/*retry_count*/ 1), None);
        assert!(error.to_string().contains("gpt-6.1-sol"));
        assert!(error.to_string().contains("gpt-6-luna"));
    }
}

#[test]
fn allowed_and_unknown_models_are_not_remapped_or_rejected() {
    for model in [
        "gpt-6.1-sol",
        "gpt-6.1-sol-fast",
        "claude-codex-gpt-6.1-sol[1m]",
        "gpt-6-luna",
        "claude-codex-gpt-6-luna[1m]",
        "gpt-6-solar",
        "gpt-6-sol-custom",
        "custom-provider-model",
        "",
    ] {
        assert_eq!(retired_codex_sol_model(model), false, "{model}");
        assert_eq!(ensure_model_not_retired(model).is_ok(), true, "{model}");
    }
}
