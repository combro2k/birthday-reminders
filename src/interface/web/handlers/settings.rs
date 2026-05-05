use std::sync::Arc;

use axum::{
    Form,
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::domain::user::User;
use crate::infrastructure::auth::session::get_csrf_token;
use crate::interface::web::server::AppState;
use crate::interface::web::templates::{ApiTokenView, ApiTokensTemplate, ProfileTemplate};

const ALLOWED_DATE_FORMATS: [&str; 3] = ["%d-%m-%Y", "%m-%d-%Y", "%Y-%m-%d"];

fn days_to_csv(days: &[i32]) -> String {
    days.iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

async fn get_user_reminder_days(state: &AppState, user: &User) -> Vec<i32> {
    state
        .user_repo
        .get_reminder_days(&user.id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            state
                .config
                .reminders
                .default_days_before
                .iter()
                .map(|&d| d as i32)
                .collect()
        })
}

fn profile_template(
    user: User,
    error: Option<String>,
    success: Option<String>,
    csrf_token: String,
    reminder_days: Vec<i32>,
) -> ProfileTemplate {
    let reminder_days_csv = days_to_csv(&reminder_days);
    ProfileTemplate {
        user,
        error,
        success,
        csrf_token,
        reminder_days,
        reminder_days_csv,
    }
}

pub async fn profile_page(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let reminder_days = get_user_reminder_days(&state, &user).await;
    Html(profile_template(user, None, None, csrf_token, reminder_days).to_string())
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
    session: Session,
    Form(form): Form<PasswordForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let reminder_days = get_user_reminder_days(&state, &user).await;

    if form.new_password != form.confirm_password {
        return Html(
            profile_template(
                user,
                Some("New passwords do not match".to_string()),
                None,
                csrf_token,
                reminder_days,
            )
            .to_string(),
        )
        .into_response();
    }

    if let Err(msg) = crate::infrastructure::auth::password::validate_password(&form.new_password) {
        return Html(
            profile_template(user, Some(msg.to_string()), None, csrf_token, reminder_days)
                .to_string(),
        )
        .into_response();
    }

    // Verify current password
    if let Some(ref hash) = user.password_hash {
        if !crate::infrastructure::auth::password::verify_password(&form.current_password, hash) {
            return Html(
                profile_template(
                    user,
                    Some("Current password is incorrect".to_string()),
                    None,
                    csrf_token,
                    reminder_days,
                )
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
            profile_template(
                user,
                None,
                Some("Password updated successfully".to_string()),
                csrf_token,
                reminder_days,
            )
            .to_string(),
        )
        .into_response(),
        Err(e) => Html(
            profile_template(user, Some(e.to_string()), None, csrf_token, reminder_days)
                .to_string(),
        )
        .into_response(),
    }
}

#[derive(Deserialize)]
pub struct ReminderDaysForm {
    pub days_before: String,
}

#[derive(Deserialize)]
pub struct DateFormatForm {
    pub date_format: String,
}

#[derive(Deserialize)]
pub struct ThemeForm {
    pub theme: String,
}

const ALLOWED_THEMES: [&str; 3] = ["light", "dark", "auto"];

pub async fn update_date_format(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<DateFormatForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let reminder_days = get_user_reminder_days(&state, &user).await;

    if !ALLOWED_DATE_FORMATS.contains(&form.date_format.as_str()) {
        return Html(
            profile_template(
                user,
                Some("Invalid date format selection".to_string()),
                None,
                csrf_token,
                reminder_days,
            )
            .to_string(),
        )
        .into_response();
    }

    match state
        .user_command_service
        .update_date_format(&user.id, &form.date_format)
        .await
    {
        Ok(()) => {
            let mut updated_user = user;
            updated_user.date_format = form.date_format;
            Html(
                profile_template(
                    updated_user,
                    None,
                    Some("Date format updated".to_string()),
                    csrf_token,
                    reminder_days,
                )
                .to_string(),
            )
            .into_response()
        }
        Err(e) => Html(
            profile_template(user, Some(e.to_string()), None, csrf_token, reminder_days)
                .to_string(),
        )
        .into_response(),
    }
}

pub async fn update_theme(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<ThemeForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let reminder_days = get_user_reminder_days(&state, &user).await;

    if !ALLOWED_THEMES.contains(&form.theme.as_str()) {
        return Html(
            profile_template(
                user,
                Some("Invalid theme selection".to_string()),
                None,
                csrf_token,
                reminder_days,
            )
            .to_string(),
        )
        .into_response();
    }

    match state
        .user_command_service
        .update_theme(&user.id, &form.theme)
        .await
    {
        Ok(()) => {
            let mut updated_user = user;
            updated_user.theme = crate::domain::user::Theme::from_str(&form.theme);
            Html(
                profile_template(
                    updated_user,
                    None,
                    Some("Theme updated".to_string()),
                    csrf_token,
                    reminder_days,
                )
                .to_string(),
            )
            .into_response()
        }
        Err(e) => Html(
            profile_template(user, Some(e.to_string()), None, csrf_token, reminder_days)
                .to_string(),
        )
        .into_response(),
    }
}

pub async fn update_reminder_days(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<ReminderDaysForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;

    // Parse comma-separated days, filter to allowed values
    let allowed: &[i32] = &[0, 1, 3, 7, 14];
    let mut days: Vec<i32> = form
        .days_before
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .filter(|d| allowed.contains(d))
        .collect();

    days.sort_unstable();
    days.dedup();
    days.reverse();

    if days.is_empty() {
        let reminder_days = get_user_reminder_days(&state, &user).await;
        return Html(
            profile_template(
                user,
                Some("Please select at least one reminder day".to_string()),
                None,
                csrf_token,
                reminder_days,
            )
            .to_string(),
        )
        .into_response();
    }

    match state
        .user_repo
        .set_reminder_days(&user.id, days.clone())
        .await
    {
        Ok(()) => Html(
            profile_template(
                user,
                None,
                Some("Reminder preferences updated".to_string()),
                csrf_token,
                days,
            )
            .to_string(),
        )
        .into_response(),
        Err(e) => Html(
            profile_template(
                user,
                Some(format!("Failed to save preferences: {}", e)),
                None,
                csrf_token,
                days,
            )
            .to_string(),
        )
        .into_response(),
    }
}

pub async fn api_tokens_page(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let tokens = state
        .user_command_service
        .list_api_tokens(&user.id, &state.db)
        .await
        .unwrap_or_default();

    let template = ApiTokensTemplate {
        user: user.clone(),
        tokens: tokens
            .into_iter()
            .map(|t| ApiTokenView::from_token_info(t, &user.date_format))
            .collect(),
        new_token: None,
        error: None,
        csrf_token,
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
    session: Session,
    Form(form): Form<NewTokenForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    match state
        .user_command_service
        .generate_api_token(&user.id, &form.name, &state.db)
        .await
    {
        Ok(plain_token) => {
            let tokens = state
                .user_command_service
                .list_api_tokens(&user.id, &state.db)
                .await
                .unwrap_or_default();
            Html(
                ApiTokensTemplate {
                    user: user.clone(),
                    tokens: tokens
                        .into_iter()
                        .map(|t| ApiTokenView::from_token_info(t, &user.date_format))
                        .collect(),
                    new_token: Some(plain_token),
                    error: None,
                    csrf_token,
                }
                .to_string(),
            )
            .into_response()
        }
        Err(e) => {
            let tokens = state
                .user_command_service
                .list_api_tokens(&user.id, &state.db)
                .await
                .unwrap_or_default();
            Html(
                ApiTokensTemplate {
                    user: user.clone(),
                    tokens: tokens
                        .into_iter()
                        .map(|t| ApiTokenView::from_token_info(t, &user.date_format))
                        .collect(),
                    new_token: None,
                    error: Some(e.to_string()),
                    csrf_token,
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
        .revoke_api_token(token_id, &user.id, &state.db)
        .await;
    Redirect::to("/settings/api-tokens")
}
