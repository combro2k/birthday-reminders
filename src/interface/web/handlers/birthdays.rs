use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::domain::user::User;
use crate::interface::web::server::AppState;
use crate::interface::web::templates::{BirthdayFormTemplate, BirthdayListTemplate, BirthdayView, DashboardTemplate};

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    let upcoming = state
        .birthday_query_service
        .get_upcoming(&user.id, 30)
        .await
        .unwrap_or_default();

    let template = DashboardTemplate {
        user,
        upcoming: upcoming.into_iter().map(BirthdayView::from).collect(),
    };
    Html(template.to_string())
}

pub async fn list_birthdays(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    let birthdays = state
        .birthday_query_service
        .list_all(&user.id)
        .await
        .unwrap_or_default();

    let template = BirthdayListTemplate {
        user,
        birthdays: birthdays.into_iter().map(BirthdayView::from).collect(),
    };
    Html(template.to_string())
}

pub async fn new_birthday_form(Extension(user): Extension<User>) -> impl IntoResponse {
    let template = BirthdayFormTemplate {
        user,
        birthday: None,
        edit_name: String::new(),
        edit_date: String::new(),
        edit_notes: String::new(),
        error: None,
    };
    Html(template.to_string())
}

#[derive(Deserialize)]
pub struct BirthdayForm {
    pub name: String,
    pub birth_date: String,
    pub notes: Option<String>,
}

#[axum::debug_handler]
pub async fn create_birthday(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Form(form): Form<BirthdayForm>,
) -> impl IntoResponse {
    let birth_date = match chrono::NaiveDate::parse_from_str(&form.birth_date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return Html(
                BirthdayFormTemplate {
                    user,
                    birthday: None,
                    edit_name: form.name,
                    edit_date: form.birth_date,
                    edit_notes: String::new(),
                    error: Some("Invalid date format. Use YYYY-MM-DD.".to_string()),
                }
                .to_string(),
            )
            .into_response();
        }
    };

    let notes = form.notes.filter(|n| !n.trim().is_empty());

    match state
        .birthday_command_service
        .add(&user.id, &form.name, birth_date, notes)
        .await
    {
        Ok(_) => Redirect::to("/birthdays").into_response(),
        Err(e) => Html(
            BirthdayFormTemplate {
                user,
                birthday: None,
                edit_name: String::new(),
                edit_date: String::new(),
                edit_notes: String::new(),
                error: Some(e.to_string()),
            }
            .to_string(),
        )
        .into_response(),
    }
}

pub async fn edit_birthday_form(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match state.birthday_query_service.get_by_id(id, &user.id).await {
        Ok(birthday) => {
            let bv = BirthdayView::from(birthday);
            let template = BirthdayFormTemplate {
                edit_name: bv.name.clone(),
                edit_date: bv.birth_date.format("%Y-%m-%d").to_string(),
                edit_notes: bv.notes.clone().unwrap_or_default(),
                user,
                birthday: Some(bv),
                error: None,
            };
            Html(template.to_string()).into_response()
        }
        Err(_) => Redirect::to("/birthdays").into_response(),
    }
}

pub async fn update_birthday(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<uuid::Uuid>,
    Form(form): Form<BirthdayForm>,
) -> impl IntoResponse {
    let birth_date = match chrono::NaiveDate::parse_from_str(&form.birth_date, "%Y-%m-%d") {
        Ok(d) => Some(d),
        Err(_) => None,
    };

    let notes = form.notes.filter(|n| !n.trim().is_empty());

    match state
        .birthday_command_service
        .update(id, &user.id, Some(form.name), birth_date, Some(notes))
        .await
    {
        Ok(_) => Redirect::to("/birthdays").into_response(),
        Err(e) => {
            let birthday = state
                .birthday_query_service
                .get_by_id(id, &user.id)
                .await
                .ok()
                .map(BirthdayView::from);
            let edit_name = birthday.as_ref().map(|b| b.name.clone()).unwrap_or_default();
            let edit_date = birthday.as_ref().map(|b| b.birth_date.format("%Y-%m-%d").to_string()).unwrap_or_default();
            let edit_notes = birthday.as_ref().and_then(|b| b.notes.clone()).unwrap_or_default();
            Html(
                BirthdayFormTemplate {
                    user,
                    birthday,
                    edit_name,
                    edit_date,
                    edit_notes,
                    error: Some(e.to_string()),
                }
                .to_string(),
            )
            .into_response()
        }
    }
}

pub async fn delete_birthday(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let _ = state.birthday_command_service.delete(id, &user.id).await;
    Redirect::to("/birthdays")
}
