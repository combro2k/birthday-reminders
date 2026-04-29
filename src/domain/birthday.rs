use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BirthdayId(pub Uuid);

impl BirthdayId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<Uuid> for BirthdayId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Birthday {
    pub id: BirthdayId,
    pub user_id: super::user::UserId,
    pub name: String,
    pub birth_date: NaiveDate,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Birthday {
    pub fn age(&self) -> u32 {
        self.age_on(Local::now().date_naive())
    }

    pub fn age_on(&self, today: NaiveDate) -> u32 {
        let mut age = today.year() - self.birth_date.year();
        if today.ordinal() < self.birth_date.ordinal() {
            age -= 1;
        }
        age.max(0) as u32
    }

    pub fn next_birthday(&self) -> NaiveDate {
        self.next_birthday_from(Local::now().date_naive())
    }

    pub fn next_birthday_from(&self, today: NaiveDate) -> NaiveDate {
        let this_year = NaiveDate::from_ymd_opt(
            today.year(),
            self.birth_date.month(),
            self.birth_date.day(),
        );

        match this_year {
            Some(date) if date >= today => date,
            _ => {
                // Try next year (handles Feb 29 gracefully)
                NaiveDate::from_ymd_opt(
                    today.year() + 1,
                    self.birth_date.month(),
                    self.birth_date.day(),
                )
                .unwrap_or_else(|| {
                    // Feb 29 in a non-leap year -> use Mar 1
                    NaiveDate::from_ymd_opt(today.year() + 1, 3, 1).unwrap()
                })
            }
        }
    }

    pub fn days_until_next(&self) -> i64 {
        self.days_until_next_from(Local::now().date_naive())
    }

    pub fn days_until_next_from(&self, today: NaiveDate) -> i64 {
        let next = self.next_birthday_from(today);
        (next - today).num_days()
    }

    /// The age they will turn on their next birthday
    pub fn turning_age(&self) -> u32 {
        self.turning_age_on(Local::now().date_naive())
    }

    pub fn turning_age_on(&self, today: NaiveDate) -> u32 {
        let next = self.next_birthday_from(today);
        (next.year() - self.birth_date.year()) as u32
    }
}
