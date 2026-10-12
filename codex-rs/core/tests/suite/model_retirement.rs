use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_login::CodexAuth;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::model_info::model_info_from_slug;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::protocol::CodexErrorInfo;
use codex_protocol::protocol::ErrorEvent;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_models_once;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use wiremock::MockServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retired_model_turn_override_emits_error_without_second_request() -> Result<()> {
    let server = MockServer::start().await;
    let allowed_slug = "gpt-6.1-sol";
    let retired_slug = "gpt-6-sol";
    let models_mock = mount_models_once(
        &server,
        ModelsResponse {
            models: vec![
                model_info_from_slug(allowed_slug),
                model_info_from_slug(retired_slug),
            ],
        },
    )
    .await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let mut builder = test_codex().with_auth(CodexAuth::create_dummy_chatgpt_auth_for_testing());
    let test = builder.build_with_auto_env(&server).await?;
    test.thread_manager
        .get_models_manager()
        .list_models(
            RefreshStrategy::OnlineIfUncached,
            codex_core::test_support::default_http_client_factory(),
        )
        .await;

    core_test_support::submit_thread_settings(
        &test.codex,
        ThreadSettingsOverrides {
            model: Some(allowed_slug.to_string()),
            ..Default::default()
        },
    )
    .await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    core_test_support::submit_thread_settings(
        &test.codex,
        ThreadSettingsOverrides {
            model: Some(retired_slug.to_string()),
            ..Default::default()
        },
    )
    .await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "hello again".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let mut retirement_errors = Vec::new();
    wait_for_event(&test.codex, |event| {
        if let EventMsg::Error(error) = event {
            retirement_errors.push(error.clone());
        }
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    assert_eq!(models_mock.single_request_path(), "/v1/models");
    assert_eq!(
        retirement_errors,
        vec![ErrorEvent {
            message: "gpt-6-sol is retired. Use gpt-6.1-sol for harder work or gpt-6-luna for routine work.".to_string(),
            codex_error_info: Some(CodexErrorInfo::Other),
            misalignment: None,
        }]
    );

    let request = response_mock.single_request();
    assert_eq!(request.body_json()["model"].as_str(), Some(allowed_slug));
    Ok(())
}
