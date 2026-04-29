use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::domain::user::User;
use crate::interface::web::server::AppState;
use crate::interface::web::templates::{ApiTokensTemplate, ApiTokenView, ProfileTemplate};

pub async fn profile_page(Extension(user): Extension<User>) -> impl IntoResponse {
    let template = ProfileTemplate {
        user,
        error: None,
        success: None,
    };
    Html(template.to_string())
}

#[derive(Deserialize)]
pub struct PasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

pub async fn update_password(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Form(form): Form<PasswordForm>,
) -> impl IntoResponse {
    if form.new_password != form.confirm_password {
        return Html(
            ProfileTemplate {
                user,
                error: Some("New passwords do not match".to_string()),
                success: None,
            }
            .to_string(),
        )
        .into_response();
    }

    if let Err(msg) = crate::infrastructure::auth::password::validate_password(&form.new_password) {
        return Html(
            ProfileTemplate {
                user,
                error: Some(msg.to_string()),
                success: None,
            }
            .to_string(),
        )
        .into_response();
    }

    // Verify current password
    if let Some(ref hash) = user.password_hash {
        if !crate::infrastructure::auth::password::verify_password(&form.current_password, hash) {
            return Html(
                ProfileTemplate {
                    user,
                    error: Some("Current password is incorrect".to_string()),
                    success: None,
                }
                .to_string(),
            )
            .into_response();
        }
    }

    match state
        .user_command_service
        .update_password(&user.id, &form.new_password)
        .await
    {
        Ok(()) => Html(
            ProfileTemplate {
                user,
                error: None,
                success: Some("Password updated successfully".to_string()),
            }
            .to_string(),
        )
        .into_response(),
        Err(e) => Html(
            ProfileTemplate {
                user,
                error: Some(e.to_string()),
                success: None,
            }
            .to_string(),
        )
        .into_response(),
    }
}

pub async fn api_tokens_page(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    let tokens = state
        .user_command_service
        .list_api_tokens(&user.id, &state.pool)
        .await
        .unwrap_or_default();

    let template = ApiTokensTemplate {
        user,
        tokens: tokens.into_iter().map(ApiTokenView::from).collect(),
        new_token: None,
        error: None,
    };
    Html(template.to_string())
}

#[derive(Deserialize)]
pub struct NewTokenForm {
    pub name: String,
}

pub async fn create_api_token(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Form(form): Form<NewTokenForm>,
) -> impl IntoResponse {
    match state
        .user_command_service
        .generate_api_token(&user.id, &form.name, &state.pool)
        .await
    {
        Ok(plain_token) => {
            let tokens = state
                .user_command_service
                .list_api_tokens(&user.id, &state.pool)
                .await
                .unwrap_or_default();
            Html(
                ApiTokensTemplate {
                    user,
                    tokens: tokens.into_iter().map(ApiTokenView::from).collect(),
                    new_token: Some(plain_token),
                    error: None,
                }
                .to_string(),
            )
            .into_response()
        }
        Err(e) => {
            let tokens = state
                .user_command_service
                .list_api_tokens(&user.id, &state.pool)
                .await
                .unwrap_or_default();
            Html(
                ApiTokensTemplate {
                    user,
                    tokens: tokens.into_iter().map(ApiTokenView::from).collect(),
                    new_token: None,
                    error: Some(e.to_string()),
                }
                .to_string(),
            )
            .into_response()
        }
    }
}

pub async fn revoke_api_token(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(token_id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let _ = state
        .user_command_service
        .revoke_api_token(token_id, &user.id, &state.pool)
        .await;
    Redirect::to("/settings/api-tokens")
}
