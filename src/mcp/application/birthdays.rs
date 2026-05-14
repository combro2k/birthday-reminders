use chrono::{Local, NaiveDate};
use rmcp::ErrorData;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::birthdays::domain::birthday::Birthday;
use crate::infrastructure::web::server::AppState;
use crate::users::domain::user::UserId;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBirthdaysInput {
    pub token: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpcomingBirthdaysInput {
    pub token: String,
    pub days: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddBirthdayInput {
    pub token: String,
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveBirthdayInput {
    pub token: String,
    pub id: String,
}

#[derive(Debug, Serialize)]
struct BirthdaysResponse {
    birthdays: Vec<BirthdayOutput>,
}

#[derive(Debug, Serialize)]
struct BirthdayOutput {
    id: String,
    name: String,
    birth_date: String,
    age: u32,
    turning_age: u32,
    days_until: i64,
    email: Option<String>,
    phone_number: Option<String>,
    address: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    country: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddBirthdayResponse {
    message: String,
    birthday: BirthdayOutput,
}

#[derive(Debug, Serialize)]
struct RemoveBirthdayNotSupportedResponse {
    supported: bool,
    message: String,
    birthday_id: String,
}

pub async fn list_birthdays(
    state: &AppState,
    input: ListBirthdaysInput,
) -> Result<String, ErrorData> {
    let user_id = resolve_user_id(state, &input.token).await?;
    let birthdays = state
        .birthday_query_service
        .list_all(&user_id)
        .await
        .map_err(|e| ErrorData::internal_error(format!("Failed to list birthdays: {e}"), None))?;

    let today = Local::now().date_naive();
    let response = BirthdaysResponse {
        birthdays: birthdays
            .iter()
            .map(|birthday| to_output(birthday, today))
            .collect(),
    };

    serde_json::to_string(&response)
        .map_err(|e| ErrorData::internal_error(format!("Failed to serialize result: {e}"), None))
}

pub async fn upcoming_birthdays(
    state: &AppState,
    input: UpcomingBirthdaysInput,
) -> Result<String, ErrorData> {
    let user_id = resolve_user_id(state, &input.token).await?;
    let days = input.days.unwrap_or(30);
    if days > 3660 {
        return Err(ErrorData::invalid_params(
            "days must be less than or equal to 3660",
            None,
        ));
    }

    let birthdays = state
        .birthday_query_service
        .get_upcoming(&user_id, days)
        .await
        .map_err(|e| {
            ErrorData::internal_error(format!("Failed to list upcoming birthdays: {e}"), None)
        })?;

    let today = Local::now().date_naive();
    let response = BirthdaysResponse {
        birthdays: birthdays
            .iter()
            .map(|birthday| to_output(birthday, today))
            .collect(),
    };

    serde_json::to_string(&response)
        .map_err(|e| ErrorData::internal_error(format!("Failed to serialize result: {e}"), None))
}

pub async fn add_birthday(state: &AppState, input: AddBirthdayInput) -> Result<String, ErrorData> {
    let user_id = resolve_user_id(state, &input.token).await?;

    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(ErrorData::invalid_params("name is required", None));
    }

    let birth_date = NaiveDate::parse_from_str(input.birth_date.trim(), "%Y-%m-%d")
        .map_err(|_| ErrorData::invalid_params("birth_date must use format YYYY-MM-DD", None))?;

    let birthday = state
        .birthday_command_service
        .add(
            &user_id,
            &name,
            birth_date,
            normalize_optional(input.email),
            normalize_optional(input.phone_number),
            normalize_optional(input.address),
            normalize_optional(input.postal_code),
            normalize_optional(input.city),
            normalize_optional(input.country),
            normalize_optional(input.notes),
        )
        .await
        .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;

    let today = Local::now().date_naive();
    let response = AddBirthdayResponse {
        message: "Birthday added".to_string(),
        birthday: to_output(&birthday, today),
    };

    serde_json::to_string(&response)
        .map_err(|e| ErrorData::internal_error(format!("Failed to serialize result: {e}"), None))
}

pub async fn remove_birthday_not_supported(
    state: &AppState,
    input: RemoveBirthdayInput,
) -> Result<String, ErrorData> {
    let _ = resolve_user_id(state, &input.token).await?;

    let response = RemoveBirthdayNotSupportedResponse {
        supported: false,
        message: "Removing birthdays via MCP is not supported. Use the web interface to delete birthdays.".to_string(),
        birthday_id: input.id,
    };

    serde_json::to_string(&response)
        .map_err(|e| ErrorData::internal_error(format!("Failed to serialize result: {e}"), None))
}

async fn resolve_user_id(state: &AppState, token: &str) -> Result<UserId, ErrorData> {
    let token = token.trim();
    if token.is_empty() {
        return Err(ErrorData::invalid_params("token is required", None));
    }

    state
        .user_command_service
        .resolve_api_token(token, &state.db)
        .await
        .map_err(|_| ErrorData::invalid_params("Invalid API token", None))
}

fn to_output(birthday: &Birthday, today: NaiveDate) -> BirthdayOutput {
    BirthdayOutput {
        id: birthday.id.0.to_string(),
        name: birthday.name.clone(),
        birth_date: birthday.birth_date.format("%Y-%m-%d").to_string(),
        age: birthday.age_on(today),
        turning_age: birthday.turning_age_on(today),
        days_until: birthday.days_until_next_from(today),
        email: birthday.email.clone(),
        phone_number: birthday.phone_number.clone(),
        address: birthday.address.clone(),
        postal_code: birthday.postal_code.clone(),
        city: birthday.city.clone(),
        country: birthday.country.clone(),
        notes: birthday.notes.clone(),
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
