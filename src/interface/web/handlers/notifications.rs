use std::sync::Arc;

use axum::{
    Form,
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::domain::notification::ChannelKind;
use crate::domain::user::User;
use crate::infrastructure::auth::session::get_csrf_token;
use crate::interface::web::server::AppState;
use crate::interface::web::templates::{
    ChannelFormTemplate, ChannelKindView, ChannelView, ChannelsTemplate,
};

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let records = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default();

    let configured_types: Vec<String> = records.iter().map(|r| r.channel_type.clone()).collect();

    let channels: Vec<ChannelView> = records.into_iter().map(ChannelView::from).collect();

    let available: Vec<ChannelKindView> = ChannelKind::implemented()
        .iter()
        .map(|k| ChannelKindView {
            kind: k.as_str().to_string(),
            display_name: k.display_name().to_string(),
            configured: configured_types.contains(&k.as_str().to_string()),
        })
        .collect();

    let template = ChannelsTemplate {
        user,
        channels,
        available,
        csrf_token,
    };
    Html(template.to_string())
}

pub async fn channel_form(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(channel_type): Path<String>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let kind = match ChannelKind::from_str(&channel_type) {
        Some(k) => k,
        None => return Redirect::to("/notifications").into_response(),
    };

    let existing = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.channel_type == channel_type);

    let template = ChannelFormTemplate {
        user,
        channel_type: channel_type.clone(),
        channel_name: kind.display_name().to_string(),
        config_json: existing
            .as_ref()
            .map(|r| serde_json::to_string_pretty(&r.config).unwrap_or_default())
            .unwrap_or_default(),
        has_existing: existing.is_some(),
        enabled: existing.as_ref().map(|r| r.enabled).unwrap_or(true),
        error: None,
        success: None,
        csrf_token,
    };
    Html(template.to_string()).into_response()
}

#[derive(Deserialize)]
pub struct ChannelConfigForm {
    pub enabled: Option<String>,
    pub config_json: String,
}

pub async fn save_channel(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(channel_type): Path<String>,
    Form(form): Form<ChannelConfigForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let kind = match ChannelKind::from_str(&channel_type) {
        Some(k) => k,
        None => return Redirect::to("/notifications").into_response(),
    };

    let config: serde_json::Value = match serde_json::from_str(&form.config_json) {
        Ok(v) => v,
        Err(e) => {
            let template = ChannelFormTemplate {
                user,
                channel_type,
                channel_name: kind.display_name().to_string(),
                config_json: form.config_json.clone(),
                has_existing: false,
                enabled: true,
                error: Some(format!("Invalid JSON config: {}", e)),
                success: None,
                csrf_token,
            };
            return Html(template.to_string()).into_response();
        }
    };

    let enabled = form.enabled.is_some();

    match state
        .notification_service
        .upsert_channel(&user.id, &channel_type, enabled, config.clone())
        .await
    {
        Ok(_) => {
            let template = ChannelFormTemplate {
                user,
                channel_type,
                channel_name: kind.display_name().to_string(),
                config_json: serde_json::to_string_pretty(&config).unwrap_or_default(),
                has_existing: true,
                enabled,
                error: None,
                success: Some("Channel saved successfully".to_string()),
                csrf_token,
            };
            Html(template.to_string()).into_response()
        }
        Err(e) => {
            let template = ChannelFormTemplate {
                user,
                channel_type,
                channel_name: kind.display_name().to_string(),
                config_json: serde_json::to_string_pretty(&config).unwrap_or_default(),
                has_existing: true,
                enabled,
                error: Some(e.to_string()),
                success: None,
                csrf_token,
            };
            Html(template.to_string()).into_response()
        }
    }
}

pub async fn test_channel(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(channel_type): Path<String>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let kind = match ChannelKind::from_str(&channel_type) {
        Some(k) => k,
        None => return Redirect::to("/notifications").into_response(),
    };

    let existing = state
        .notification_service
        .list_channels(&user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.channel_type == channel_type);

    let (error, success) = match state
        .notification_service
        .test_channel(&user.id, &channel_type)
        .await
    {
        Ok(()) => (
            None,
            Some("Test notification sent successfully!".to_string()),
        ),
        Err(e) => (Some(format!("Test failed: {}", e)), None),
    };

    let template = ChannelFormTemplate {
        user,
        channel_type,
        channel_name: kind.display_name().to_string(),
        config_json: existing
            .as_ref()
            .map(|r| serde_json::to_string_pretty(&r.config).unwrap_or_default())
            .unwrap_or_default(),
        has_existing: existing.is_some(),
        enabled: existing.as_ref().map(|r| r.enabled).unwrap_or(true),
        error,
        success,
        csrf_token,
    };
    Html(template.to_string()).into_response()
}

pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(channel_type): Path<String>,
) -> impl IntoResponse {
    let _ = state
        .notification_service
        .delete_channel(&user.id, &channel_type)
        .await;
    Redirect::to("/notifications")
}
