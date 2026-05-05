use askama::Template;
use chrono::NaiveDate;

use crate::birthdays::domain::birthday::Birthday;
use crate::infrastructure::web::templates::AppVersion;
use crate::users::domain::user::User;

#[derive(Template)]
#[template(path = "birthdays/index.html")]
pub struct DashboardTemplate {
    pub user: User,
    pub upcoming: Vec<BirthdayView>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "birthdays/list.html")]
pub struct BirthdayListTemplate {
    pub user: User,
    pub birthdays: Vec<BirthdayView>,
    pub current_sort: String,
    pub is_desc: bool,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "birthdays/form.html")]
pub struct BirthdayFormTemplate {
    pub user: User,
    pub birthday: Option<BirthdayView>,
    pub edit_name: String,
    pub edit_date: String,
    pub edit_email: String,
    pub edit_phone_number: String,
    pub edit_address: String,
    pub edit_postal_code: String,
    pub edit_city: String,
    pub edit_country: String,
    pub edit_notes: String,
    pub error: Option<String>,
    pub csrf_token: String,
}

impl AppVersion for DashboardTemplate {}
impl AppVersion for BirthdayListTemplate {}
impl AppVersion for BirthdayFormTemplate {}

// ---- View Models ----

#[derive(Debug, Clone)]
pub struct BirthdayView {
    pub id: uuid::Uuid,
    pub name: String,
    pub birth_date: NaiveDate,
    pub birth_date_str: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub notes: Option<String>,
    pub age: u32,
    pub turning_age: u32,
    pub days_until: i64,
}

impl BirthdayView {
    pub fn from_birthday(b: Birthday, date_format: &str) -> Self {
        let today = chrono::Local::now().date_naive();
        let format = if date_format.is_empty() {
            "%Y-%m-%d"
        } else {
            date_format
        };

        Self {
            id: b.id.0,
            name: b.name.clone(),
            birth_date: b.birth_date,
            birth_date_str: b.birth_date.format(format).to_string(),
            email: b.email.clone(),
            phone_number: b.phone_number.clone(),
            address: b.address.clone(),
            postal_code: b.postal_code.clone(),
            city: b.city.clone(),
            country: b.country.clone(),
            notes: b.notes.clone(),
            age: b.age_on(today),
            turning_age: b.turning_age_on(today),
            days_until: b.days_until_next_from(today),
        }
    }
}
