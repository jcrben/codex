use super::*;
use codex_http_client::OutboundProxyPolicy;
use pretty_assertions::assert_eq;

fn catalog() -> StaticModelsManager {
    let models = [
        "gpt-6-sol",
        "claude-codex-gpt-6-sol-fast[1m]",
        "gpt-6.1-sol",
        "gpt-6-luna",
        "custom-model",
    ]
    .into_iter()
    .enumerate()
    .map(|(priority, slug)| {
        let mut model = crate::model_info::model_info_from_slug(slug);
        model.visibility = ModelVisibility::List;
        model.priority = priority as i32;
        model
    })
    .collect();
    StaticModelsManager::new(/*auth_manager*/ None, ModelsResponse { models })
}

#[test]
fn retirement_picker_filters_cached_custom_forms_before_default_selection() {
    let manager = catalog();
    let raw = manager.try_get_remote_models().expect("static catalog");
    let available = manager.build_available_models(raw.clone());
    assert_eq!(
        available
            .iter()
            .map(|model| model.model.as_str())
            .collect::<Vec<_>>(),
        vec!["gpt-6.1-sol", "gpt-6-luna", "custom-model"]
    );
    assert_eq!(
        available
            .iter()
            .filter(|model| model.is_default)
            .map(|model| model.model.as_str())
            .collect::<Vec<_>>(),
        vec!["gpt-6.1-sol"]
    );
    assert_eq!(
        manager
            .try_get_remote_models()
            .expect("raw catalog remains unchanged"),
        raw
    );
}

#[tokio::test]
async fn retirement_explicit_selection_is_not_silently_remapped() {
    let manager = catalog();
    let factory = HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);
    for slug in [
        "gpt-6-sol",
        "claude-codex-gpt-6-sol-fast[1m]",
        " GPT-6-SOL-FAST-FAST ",
    ] {
        for allow_provider_model_fallback in [false, true] {
            assert_eq!(
                manager
                    .get_default_model(
                        &Some(slug.to_string()),
                        allow_provider_model_fallback,
                        RefreshStrategy::Offline,
                        factory.clone(),
                    )
                    .await,
                slug
            );
        }
    }
    assert_eq!(
        manager
            .get_default_model(
                &Some("unknown-model".to_string()),
                /*allow_provider_model_fallback*/ true,
                RefreshStrategy::Offline,
                factory.clone(),
            )
            .await,
        "gpt-6.1-sol"
    );
    assert_eq!(
        manager
            .get_default_model(
                &Some("unknown-model".to_string()),
                /*allow_provider_model_fallback*/ false,
                RefreshStrategy::Offline,
                factory,
            )
            .await,
        "unknown-model"
    );
}
