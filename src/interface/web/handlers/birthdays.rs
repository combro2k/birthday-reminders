use std::sync::Arc;

use axum::{
    Form,
    extract::{Extension, Path, Query, State},
    response::{Html, IntoResponse, Redirect},
};
use chrono::Local;
use serde::Deserialize;
use tower_sessions::Session;

use crate::domain::user::User;
use crate::infrastructure::auth::session::get_csrf_token;
use crate::interface::web::server::AppState;
use crate::interface::web::templates::{
    BirthdayFormTemplate, BirthdayListTemplate, BirthdayView, DashboardTemplate,
};

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let upcoming = state
        .birthday_query_service
        .get_upcoming(&user.id, 30)
        .await
        .unwrap_or_default();

    let upcoming_views = upcoming
        .into_iter()
        .map(|b| BirthdayView::from_birthday(b, &user.date_format))
        .collect();

    let template = DashboardTemplate {
        user,
        upcoming: upcoming_views,
        csrf_token,
    };
    Html(template.to_string())
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub sort: Option<String>,
    pub desc: Option<bool>,
}

pub async fn list_birthdays(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let mut birthdays = state
        .birthday_query_service
        .list_all(&user.id)
        .await
        .unwrap_or_default();

    let current_sort = query.sort.as_deref().unwrap_or("name");
    let is_desc = query.desc.unwrap_or(false);
    let today = Local::now().date_naive();

    birthdays.sort_by(|a, b| {
        let res = match current_sort {
            "date" => a
                .days_until_next_from(today)
                .cmp(&b.days_until_next_from(today))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            "age" => a
                .birth_date
                .cmp(&b.birth_date)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            _ => a
                .name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| {
                    a.days_until_next_from(today)
                        .cmp(&b.days_until_next_from(today))
                }),
        };

        if is_desc { res.reverse() } else { res }
    });

    let birthday_views = birthdays
        .into_iter()
        .map(|b| BirthdayView::from_birthday(b, &user.date_format))
        .collect();

    let template = BirthdayListTemplate {
        user,
        birthdays: birthday_views,
        current_sort: current_sort.to_string(),
        is_desc,
        csrf_token,
    };
    Html(template.to_string())
}

pub async fn new_birthday_form(
    Extension(user): Extension<User>,
    session: Session,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let template = BirthdayFormTemplate {
        user,
        birthday: None,
        edit_name: String::new(),
        edit_date: String::new(),
        edit_email: String::new(),
        edit_phone_number: String::new(),
        edit_address: String::new(),
        edit_postal_code: String::new(),
        edit_city: String::new(),
        edit_country: String::new(),
        edit_notes: String::new(),
        error: None,
        csrf_token,
    };
    Html(template.to_string())
}

#[derive(Deserialize)]
pub struct BirthdayForm {
    pub name: String,
    pub birth_date: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
}

#[axum::debug_handler]
pub async fn create_birthday(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Form(form): Form<BirthdayForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let birth_date = match chrono::NaiveDate::parse_from_str(&form.birth_date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            let edit_notes = form.notes.unwrap_or_default();
            return Html(
                BirthdayFormTemplate {
                    user,
                    birthday: None,
                    edit_name: form.name,
                    edit_date: form.birth_date,
                    edit_email: form.email.unwrap_or_default(),
                    edit_phone_number: form.phone_number.unwrap_or_default(),
                    edit_address: form.address.unwrap_or_default(),
                    edit_postal_code: form.postal_code.unwrap_or_default(),
                    edit_city: form.city.unwrap_or_default(),
                    edit_country: form.country.unwrap_or_default(),
                    edit_notes,
                    error: Some("Invalid date format.".to_string()),
                    csrf_token,
                }
                .to_string(),
            )
            .into_response();
        }
    };

    let email = form.email.clone().filter(|n| !n.trim().is_empty());
    let phone_number = form.phone_number.clone().filter(|n| !n.trim().is_empty());
    let address = form.address.clone().filter(|n| !n.trim().is_empty());
    let postal_code = form.postal_code.clone().filter(|n| !n.trim().is_empty());
    let city = form.city.clone().filter(|n| !n.trim().is_empty());
    let country = form.country.clone().filter(|n| !n.trim().is_empty());
    let notes = form.notes.clone().filter(|n| !n.trim().is_empty());

    match state
        .birthday_command_service
        .add(
            &user.id,
            &form.name,
            birth_date,
            email,
            phone_number,
            address,
            postal_code,
            city,
            country,
            notes,
        )
        .await
    {
        Ok(_) => Redirect::to("/birthdays").into_response(),
        Err(e) => Html(
            BirthdayFormTemplate {
                user,
                birthday: None,
                edit_name: form.name,
                edit_date: form.birth_date,
                edit_email: form.email.unwrap_or_default(),
                edit_phone_number: form.phone_number.unwrap_or_default(),
                edit_address: form.address.unwrap_or_default(),
                edit_postal_code: form.postal_code.unwrap_or_default(),
                edit_city: form.city.unwrap_or_default(),
                edit_country: form.country.unwrap_or_default(),
                edit_notes: form.notes.unwrap_or_default(),
                error: Some(e.to_string()),
                csrf_token,
            }
            .to_string(),
        )
        .into_response(),
    }
}

pub async fn edit_birthday_form(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    match state.birthday_query_service.get_by_id(id, &user.id).await {
        Ok(birthday) => {
            let bv = BirthdayView::from_birthday(birthday, &user.date_format);
            let template = BirthdayFormTemplate {
                edit_name: bv.name.clone(),
                edit_date: bv.birth_date.format("%Y-%m-%d").to_string(),
                edit_email: bv.email.clone().unwrap_or_default(),
                edit_phone_number: bv.phone_number.clone().unwrap_or_default(),
                edit_address: bv.address.clone().unwrap_or_default(),
                edit_postal_code: bv.postal_code.clone().unwrap_or_default(),
                edit_city: bv.city.clone().unwrap_or_default(),
                edit_country: bv.country.clone().unwrap_or_default(),
                edit_notes: bv.notes.clone().unwrap_or_default(),
                user,
                birthday: Some(bv),
                error: None,
                csrf_token,
            };
            Html(template.to_string()).into_response()
        }
        Err(_) => Redirect::to("/birthdays").into_response(),
    }
}

pub async fn update_birthday(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    session: Session,
    Path(id): Path<uuid::Uuid>,
    Form(form): Form<BirthdayForm>,
) -> impl IntoResponse {
    let csrf_token = get_csrf_token(&session).await;
    let birth_date = match chrono::NaiveDate::parse_from_str(&form.birth_date, "%Y-%m-%d") {
        Ok(d) => Some(d),
        Err(_) => {
            let birthday = state
                .birthday_query_service
                .get_by_id(id, &user.id)
                .await
                .ok()
                .map(|b| BirthdayView::from_birthday(b, &user.date_format));

            return Html(
                BirthdayFormTemplate {
                    user,
                    birthday,
                    edit_name: form.name,
                    edit_date: form.birth_date,
                    edit_email: form.email.unwrap_or_default(),
                    edit_phone_number: form.phone_number.unwrap_or_default(),
                    edit_address: form.address.unwrap_or_default(),
                    edit_postal_code: form.postal_code.unwrap_or_default(),
                    edit_city: form.city.unwrap_or_default(),
                    edit_country: form.country.unwrap_or_default(),
                    edit_notes: form.notes.unwrap_or_default(),
                    error: Some("Invalid date format.".to_string()),
                    csrf_token,
                }
                .to_string(),
            )
            .into_response();
        }
    };

    let email = form.email.clone().filter(|n| !n.trim().is_empty());
    let phone_number = form.phone_number.clone().filter(|n| !n.trim().is_empty());
    let address = form.address.clone().filter(|n| !n.trim().is_empty());
    let postal_code = form.postal_code.clone().filter(|n| !n.trim().is_empty());
    let city = form.city.clone().filter(|n| !n.trim().is_empty());
    let country = form.country.clone().filter(|n| !n.trim().is_empty());
    let notes = form.notes.clone().filter(|n| !n.trim().is_empty());

    match state
        .birthday_command_service
        .update(
            id,
            &user.id,
            Some(form.name.clone()),
            birth_date,
            Some(email),
            Some(phone_number),
            Some(address),
            Some(postal_code),
            Some(city),
            Some(country),
            Some(notes),
        )
        .await
    {
        Ok(_) => Redirect::to("/birthdays").into_response(),
        Err(e) => {
            let birthday = state
                .birthday_query_service
                .get_by_id(id, &user.id)
                .await
                .ok()
                .map(|b| BirthdayView::from_birthday(b, &user.date_format));
            let edit_name = form.name;
            let edit_date = form.birth_date;
            let edit_email = form.email.unwrap_or_default();
            let edit_phone_number = form.phone_number.unwrap_or_default();
            let edit_address = form.address.unwrap_or_default();
            let edit_postal_code = form.postal_code.unwrap_or_default();
            let edit_city = form.city.unwrap_or_default();
            let edit_country = form.country.unwrap_or_default();
            let edit_notes = form.notes.unwrap_or_default();

            Html(
                BirthdayFormTemplate {
                    user,
                    birthday,
                    edit_name,
                    edit_date,
                    edit_email,
                    edit_phone_number,
                    edit_address,
                    edit_postal_code,
                    edit_city,
                    edit_country,
                    edit_notes,
                    error: Some(e.to_string()),
                    csrf_token,
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
