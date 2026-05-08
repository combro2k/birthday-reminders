use std::sync::Arc;

use axum::{
    Form,
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::auth::infrastructure::session::get_csrf_token;
use crate::infrastructure::web::server::AppState;
use crate::users::domain::user::User;
use crate::users::presentation::templates::{ApiTokenView, ApiTokensTemplate, ProfileTemplate};

const ALLOWED_DATE_FORMATS: [&str; 3] = ["%d-%m-%Y", "%m-%d-%Y", "%Y-%m-%d"];
const ALLOWED_DASHBOARD_WINDOWS: [u32; 5] = [30, 45, 60, 75, 90];
const ALLOWED_BIRTHDAY_SORT_FIELDS: [&str; 3] = ["name", "date", "age"];
const ALLOWED_REMINDER_DAYS: [i32; 10] = [0, 1, 3, 7, 14, 30, 45, 60, 75, 90];

fn days_to_csv(days: &[i32]) -> String {
    days.iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn is_allowed_dashboard_window(days: u32) -> bool {
    ALLOWED_DASHBOARD_WINDOWS.contains(&days)
}

fn is_allowed_birthday_sort_field(field: &str) -> bool {
    ALLOWED_BIRTHDAY_SORT_FIELDS.contains(&field)
}

fn parse_sort_direction(direction: &str) -> Option<bool> {
    match direction {
        "asc" => Some(false),
        "desc" => Some(true),
        _ => None,
    }
}

fn parse_and_normalize_reminder_days(days_before: &str) -> Vec<i32> {
    let mut days: Vec<i32> = days_before
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .filter(|d| ALLOWED_REMINDER_DAYS.contains(d))
        .collect();

    days.sort_unstable();
    days.dedup();
    days.reverse();
    days
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

    if let Err(msg) = crate::auth::infrastructure::password::validate_password(&form.new_password) {
        return Html(
            profile_template(user, Some(msg.to_string()), None, csrf_token, reminder_days)
                .to_string(),
        )
        .into_response();
    }

    // Verify current password
    if let Some(ref hash) = user.password_hash
        && !crate::auth::infrastructure::password::verify_password(&form.current_password, hash)
    {
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

#[derive(Deserialize)]
pub struct DashboardWindowForm {
    pub dashboard_upcoming_days: u32,
}

#[derive(Deserialize)]
pub struct BirthdaySortForm {
    pub birthday_sort_field: String,
    pub birthday_sort_direction: String,
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
            updated_user.theme = crate::users::domain::user::Theme::from_str(&form.theme);
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

pub async fn update_dashboard_window(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<DashboardWindowForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let reminder_days = get_user_reminder_days(&state, &user).await;

    if !is_allowed_dashboard_window(form.dashboard_upcoming_days) {
        return Html(
            profile_template(
                user,
                Some("Invalid dashboard window selection".to_string()),
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
        .update_dashboard_upcoming_days(&user.id, form.dashboard_upcoming_days)
        .await
    {
        Ok(()) => {
            let mut updated_user = user;
            updated_user.dashboard_upcoming_days = form.dashboard_upcoming_days;
            Html(
                profile_template(
                    updated_user,
                    None,
                    Some("Dashboard window updated".to_string()),
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

pub async fn update_birthday_sort_preferences(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<BirthdaySortForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let reminder_days = get_user_reminder_days(&state, &user).await;

    if !is_allowed_birthday_sort_field(&form.birthday_sort_field) {
        return Html(
            profile_template(
                user,
                Some("Invalid birthday sorting field".to_string()),
                None,
                csrf_token,
                reminder_days,
            )
            .to_string(),
        )
        .into_response();
    }

    let sort_desc = match parse_sort_direction(&form.birthday_sort_direction) {
        Some(sort_desc) => sort_desc,
        None => {
            return Html(
                profile_template(
                    user,
                    Some("Invalid birthday sort direction".to_string()),
                    None,
                    csrf_token,
                    reminder_days,
                )
                .to_string(),
            )
            .into_response();
        }
    };

    match state
        .user_command_service
        .update_birthday_sort_preferences(&user.id, &form.birthday_sort_field, sort_desc)
        .await
    {
        Ok(()) => {
            let mut updated_user = user;
            updated_user.birthday_sort_field = form.birthday_sort_field;
            updated_user.birthday_sort_desc = sort_desc;
            Html(
                profile_template(
                    updated_user,
                    None,
                    Some("Birthday sorting preference updated".to_string()),
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

    let days = parse_and_normalize_reminder_days(&form.days_before);

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

#[cfg(test)]
mod tests {
    use super::{
        is_allowed_birthday_sort_field, is_allowed_dashboard_window,
        parse_and_normalize_reminder_days, parse_sort_direction,
    };

    #[test]
    fn dashboard_window_allows_expected_values() {
        for value in [30_u32, 45, 60, 75, 90] {
            assert!(is_allowed_dashboard_window(value));
        }
        assert!(!is_allowed_dashboard_window(15));
        assert!(!is_allowed_dashboard_window(120));
    }

    #[test]
    fn birthday_sort_field_allows_expected_values() {
        assert!(is_allowed_birthday_sort_field("date"));
        assert!(is_allowed_birthday_sort_field("name"));
        assert!(is_allowed_birthday_sort_field("age"));
        assert!(!is_allowed_birthday_sort_field("days_until"));
        assert!(!is_allowed_birthday_sort_field(""));
    }

    #[test]
    fn sort_direction_parses_expected_values() {
        assert_eq!(parse_sort_direction("asc"), Some(false));
        assert_eq!(parse_sort_direction("desc"), Some(true));
        assert_eq!(parse_sort_direction("ASC"), None);
        assert_eq!(parse_sort_direction("invalid"), None);
    }

    #[test]
    fn reminder_days_parsing_filters_dedups_and_sorts_descending() {
        let parsed = parse_and_normalize_reminder_days("90,30,30,7,2,1,45,abc,0,75");
        assert_eq!(parsed, vec![90, 75, 45, 30, 7, 1, 0]);
    }

    #[test]
    fn reminder_days_parsing_returns_empty_for_invalid_input() {
        let parsed = parse_and_normalize_reminder_days("2,5,10,invalid");
        assert!(parsed.is_empty());
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
