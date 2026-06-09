use std::sync::Arc;

use axum::{
    Extension, Form,
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::auth::infrastructure::session::{clear_session, get_csrf_token, set_user_id};
use crate::auth::presentation::templates::{LoginTemplate, RegisterTemplate};
use crate::infrastructure::web::middleware::ClientInfo;
use crate::infrastructure::web::server::AppState;

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct LoginQuery {
    pub next: Option<String>,
}

async fn build_login_template(
    state: &AppState,
    session: &Session,
    error: Option<String>,
    next: Option<String>,
) -> LoginTemplate {
    LoginTemplate {
        error,
        oidc_enabled: state.oidc_client.is_some(),
        oidc_provider_name: state
            .config
            .auth
            .oidc
            .as_ref()
            .map(|o| o.provider_name.clone())
            .unwrap_or_default(),
        registration_enabled: state.config.auth.allow_registration,
        csrf_token: get_csrf_token(session).await,
        next,
    }
}

pub async fn login_page(
    State(state): State<Arc<AppState>>,
    session: Session,
    Query(params): Query<LoginQuery>,
) -> impl IntoResponse {
    let template = build_login_template(&state, &session, None, params.next).await;
    Html(template.to_string())
}

pub async fn login_submit(
    State(state): State<Arc<AppState>>,
    session: Session,
    client_info: Option<Extension<ClientInfo>>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let client_ip = client_info.map(|Extension(ci)| ci.ip);

    match state
        .auth_service
        .login_local(&form.username, &form.password)
        .await
    {
        Ok(user) => {
            if set_user_id(&session, &user.id).await.is_err() {
                return Redirect::to("/auth/login").into_response();
            }
            tracing::debug!(client_ip = ?client_ip, username = %form.username, "login succeeded");
            let target = safe_next_path(form.next.as_deref()).unwrap_or("/");
            Redirect::to(target).into_response()
        }
        Err(e) => {
            tracing::warn!(client_ip = ?client_ip, username = %form.username, "login failed");
            let template =
                build_login_template(&state, &session, Some(e.to_string()), form.next).await;
            Html(template.to_string()).into_response()
        }
    }
}

pub async fn logout(session: Session) -> impl IntoResponse {
    clear_session(&session).await;
    Redirect::to("/auth/login")
}

// ---- OIDC ----

const OIDC_STATE_KEY: &str = "oidc_flow_state";

pub async fn oidc_login(State(state): State<Arc<AppState>>, session: Session) -> impl IntoResponse {
    match state.auth_service.initiate_oidc() {
        Ok((url, flow_state)) => {
            let state_json = serde_json::to_string(&flow_state).unwrap_or_default();
            let _ = session.insert(OIDC_STATE_KEY, state_json).await;
            Redirect::to(&url).into_response()
        }
        Err(e) => {
            let template =
                build_login_template(&state, &session, Some(format!("OIDC error: {}", e)), None)
                    .await;
            Html(template.to_string()).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct OidcCallback {
    pub code: String,
    pub state: String,
}

pub async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    session: Session,
    client_info: Option<Extension<ClientInfo>>,
    Query(params): Query<OidcCallback>,
) -> impl IntoResponse {
    let client_ip = client_info.map(|Extension(ci)| ci.ip);
    // Retrieve flow state from session
    let flow_state_json: Option<String> = session.get(OIDC_STATE_KEY).await.ok().flatten();
    let _ = session.remove::<String>(OIDC_STATE_KEY).await;

    let flow_state_json = match flow_state_json {
        Some(s) => s,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let flow_state: crate::auth::infrastructure::oidc::OidcFlowState =
        match serde_json::from_str(&flow_state_json) {
            Ok(s) => s,
            Err(_) => return Redirect::to("/auth/login").into_response(),
        };

    // Verify CSRF
    if params.state != flow_state.csrf_token {
        tracing::warn!(client_ip = ?client_ip, "OIDC callback CSRF mismatch");
        return Redirect::to("/auth/login").into_response();
    }

    match state
        .auth_service
        .handle_oidc_callback(&params.code, &flow_state)
        .await
    {
        Ok(user) => {
            if set_user_id(&session, &user.id).await.is_err() {
                return Redirect::to("/auth/login").into_response();
            }
            tracing::debug!(client_ip = ?client_ip, user_id = ?user.id, "OIDC login succeeded");
            Redirect::to("/").into_response()
        }
        Err(e) => {
            tracing::warn!(client_ip = ?client_ip, error = %e, "OIDC login failed");
            let template = build_login_template(&state, &session, Some(e.to_string()), None).await;
            Html(template.to_string()).into_response()
        }
    }
}

fn safe_next_path(next: Option<&str>) -> Option<&str> {
    let candidate = next?.trim();
    if candidate.starts_with('/') && !candidate.starts_with("//") {
        Some(candidate)
    } else {
        None
    }
}

// ---- Registration ----

#[derive(Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub password_confirm: String,
}

pub async fn register_page(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> impl IntoResponse {
    if !state.config.auth.allow_registration {
        return Redirect::to("/auth/login").into_response();
    }
    let csrf_token = get_csrf_token(&session).await;
    Html(
        RegisterTemplate {
            error: None,
            csrf_token,
        }
        .to_string(),
    )
    .into_response()
}

pub async fn register_submit(
    State(state): State<Arc<AppState>>,
    session: Session,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    if !state.config.auth.allow_registration {
        return Redirect::to("/auth/login").into_response();
    }

    let csrf_token = get_csrf_token(&session).await;

    if form.password != form.password_confirm {
        return Html(
            RegisterTemplate {
                error: Some("Passwords do not match".to_string()),
                csrf_token,
            }
            .to_string(),
        )
        .into_response();
    }

    if let Err(msg) = crate::auth::infrastructure::password::validate_password(&form.password) {
        return Html(
            RegisterTemplate {
                error: Some(msg.to_string()),
                csrf_token,
            }
            .to_string(),
        )
        .into_response();
    }

    match state
        .user_command_service
        .create_user(
            &form.username,
            &form.email,
            &form.password,
            crate::users::domain::user::Role::User,
        )
        .await
    {
        Ok(user) => {
            if set_user_id(&session, &user.id).await.is_err() {
                return Redirect::to("/auth/login").into_response();
            }
            Redirect::to("/").into_response()
        }
        Err(e) => Html(
            RegisterTemplate {
                error: Some(e.to_string()),
                csrf_token,
            }
            .to_string(),
        )
        .into_response(),
    }
}
