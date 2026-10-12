//! Local request/admission tests: no server, provider request, or auth store.

use super::*;
use codex_http_client::OutboundProxyPolicy;
use codex_model_provider_info::create_oss_provider_with_base_url;
use codex_protocol::protocol::RealtimeOutputModality;
use codex_protocol::protocol::RealtimeVoicesList;
use pretty_assertions::assert_eq;

const RETIRED: [&str; 8] = [
    "gpt-6-sol",
    "gpt-6-sol-fast",
    "gpt-6-sol-fast-fast",
    "gpt-6-sol[1m]",
    "claude-codex-gpt-6-sol",
    "claude-codex-gpt-6-sol-fast[1m]",
    " GPT-6-SOL ",
    "\tCLAUDE-CODEX-GPT-6-SOL-FAST-FAST[1M] ",
];

fn client() -> ModelClient {
    ModelClient::new(
        /*auth_manager*/ None,
        AgentIdentityAuthPolicy::JwtOnly,
        ThreadId::new(),
        create_oss_provider_with_base_url("http://127.0.0.1:1/v1", WireApi::Responses),
        SessionSource::Cli,
        "retirement-test".to_string(),
        /*model_verbosity*/ None,
        /*content_item_kinds_enabled*/ true,
        /*reasoning_effort_override_enabled*/ false,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
        /*concurrent_reasoning_summaries_enabled*/ false,
        /*attestation_provider*/ None,
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        WorkspaceRoutingContext::new("http://127.0.0.1:1".to_string()),
        Vec::new(),
    )
}

fn metadata() -> CodexResponsesMetadata {
    CodexResponsesMetadata::new(
        "retirement-installation".to_string(),
        "retirement-session".to_string(),
        "retirement-thread".to_string(),
        "retirement-window".to_string(),
    )
}

fn telemetry() -> SessionTelemetry {
    SessionTelemetry::new(
        ThreadId::new(),
        "retirement-test",
        "retirement-test",
        /*account_id*/ None,
        /*account_email*/ None,
        /*auth_mode*/ None,
        "retirement-test".to_string(),
        /*log_user_prompts*/ false,
        "retirement-test".to_string(),
        SessionSource::Cli,
    )
}

fn assert_retired<T>(result: Result<T>) {
    let error = result.err().expect("retired selection must refuse locally");
    assert!(error.to_string().contains("gpt-6-sol is retired"));
    assert_eq!(error.retry_delay(/*retry_count*/ 1), None);
}

#[test]
fn retirement_responses_payload_rejects_all_reviewed_forms() {
    let client = client();
    for slug in RETIRED {
        let model = codex_models_manager::model_info::model_info_from_slug(slug);
        for include_internal in [false, true] {
            assert_retired(client.build_responses_request(
                &Prompt::default(),
                &model,
                /*effort*/ None,
                ReasoningSummaryConfig::None,
                /*service_tier*/ None,
                &metadata(),
                include_internal,
            ));
        }
    }
}

#[test]
fn retirement_allowed_payload_preserves_exact_selected_model() {
    let client = client();
    for slug in [
        "gpt-6.1-sol",
        "gpt-6-luna",
        "claude-codex-gpt-6.1-sol[1m]",
        "custom-model",
    ] {
        let model = codex_models_manager::model_info::model_info_from_slug(slug);
        let request = client
            .build_responses_request(
                &Prompt::default(),
                &model,
                /*effort*/ None,
                ReasoningSummaryConfig::None,
                /*service_tier*/ None,
                &metadata(),
                /*include_internal*/ false,
            )
            .expect("allowed payload remains constructible");
        assert_eq!(request.model, slug);
    }
}

#[tokio::test]
async fn retirement_memory_summary_refuses_before_even_empty_input_shortcut() {
    let client = client();
    for slug in RETIRED {
        let model = codex_models_manager::model_info::model_info_from_slug(slug);
        assert_retired(
            client
                .summarize_memories(Vec::new(), &model, /*effort*/ None, &telemetry())
                .await,
        );
    }
    for slug in ["gpt-6.1-sol", "gpt-6-luna"] {
        let model = codex_models_manager::model_info::model_info_from_slug(slug);
        assert_eq!(
            client
                .summarize_memories(Vec::new(), &model, /*effort*/ None, &telemetry())
                .await
                .is_ok(),
            true
        );
    }
}

#[tokio::test]
async fn retirement_preconnect_and_prewarm_refuse_before_transport_shortcuts() {
    let client = client();
    assert_eq!(client.responses_websocket_enabled(), false);
    for slug in RETIRED {
        let model = codex_models_manager::model_info::model_info_from_slug(slug);
        let mut session = client.new_session();
        assert_retired(
            session
                .preconnect_websocket(
                    &model,
                    /*service_tier*/ None,
                    &telemetry(),
                    &metadata(),
                )
                .await,
        );
        assert_retired(
            session
                .prewarm_websocket(
                    &Prompt::default(),
                    &model,
                    &telemetry(),
                    /*effort*/ None,
                    ReasoningSummaryConfig::None,
                    /*service_tier*/ None,
                    &metadata(),
                )
                .await,
        );
    }
}

#[tokio::test]
async fn retirement_late_selection_refuses_at_stream_entry_without_request() {
    let client = client();
    let allowed = codex_models_manager::model_info::model_info_from_slug("gpt-6.1-sol");
    assert_eq!(
        client
            .build_responses_request(
                &Prompt::default(),
                &allowed,
                /*effort*/ None,
                ReasoningSummaryConfig::None,
                /*service_tier*/ None,
                &metadata(),
                /*include_internal*/ false,
            )
            .is_ok(),
        true
    );
    let mut session = client.new_session();
    for slug in RETIRED {
        let model = codex_models_manager::model_info::model_info_from_slug(slug);
        assert_retired(
            session
                .stream(
                    &Prompt::default(),
                    &model,
                    &telemetry(),
                    /*effort*/ None,
                    ReasoningSummaryConfig::None,
                    /*service_tier*/ None,
                    &metadata(),
                    &InferenceTraceContext::disabled(),
                )
                .await,
        );
    }
}

#[tokio::test]
async fn retirement_direct_realtime_call_refuses_before_auth_or_transport() {
    let client = client();
    for slug in RETIRED {
        let config = ApiRealtimeSessionConfig {
            instructions: String::new(),
            initial_items: Vec::new(),
            delegation_ack_filler: None,
            model: Some(slug.to_string()),
            session_id: None,
            event_parser: codex_api::RealtimeEventParser::V1,
            session_mode: codex_api::RealtimeSessionMode::Conversational,
            output_modality: RealtimeOutputModality::Audio,
            voice: RealtimeVoicesList::builtin().default_v1,
        };
        assert_retired(
            client
                .create_realtime_call_with_headers(
                    String::new(),
                    config,
                    ApiHeaderMap::new(),
                    /*api_provider_override*/ None,
                )
                .await,
        );
    }
}
