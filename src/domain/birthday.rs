use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BirthdayId(pub Uuid);

impl BirthdayId {
    #[cfg(test)]
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
    pub fn age_on(&self, today: NaiveDate) -> u32 {
        let mut age = today.year() - self.birth_date.year();
        if today.ordinal() < self.birth_date.ordinal() {
            age -= 1;
        }
        age.max(0) as u32
    }

    pub fn next_birthday_from(&self, today: NaiveDate) -> NaiveDate {
        let this_year =
            NaiveDate::from_ymd_opt(today.year(), self.birth_date.month(), self.birth_date.day());

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

    pub fn days_until_next_from(&self, today: NaiveDate) -> i64 {
        let next = self.next_birthday_from(today);
        (next - today).num_days()
    }

    pub fn turning_age_on(&self, today: NaiveDate) -> u32 {
        let next = self.next_birthday_from(today);
        (next.year() - self.birth_date.year()) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::UserId;

    fn make_birthday(year: i32, month: u32, day: u32) -> Birthday {
        Birthday {
            id: BirthdayId::new(),
            user_id: UserId::new(),
            name: "Test".to_string(),
            birth_date: NaiveDate::from_ymd_opt(year, month, day).unwrap(),
            notes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn age_before_birthday_this_year() {
        let b = make_birthday(1990, 12, 25);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.age_on(today), 35);
    }

    #[test]
    fn age_after_birthday_this_year() {
        let b = make_birthday(1990, 3, 10);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.age_on(today), 36);
    }

    #[test]
    fn age_on_birthday() {
        let b = make_birthday(1990, 6, 1);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.age_on(today), 36);
    }

    #[test]
    fn next_birthday_in_future_this_year() {
        let b = make_birthday(1990, 12, 25);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(
            b.next_birthday_from(today),
            NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()
        );
    }

    #[test]
    fn next_birthday_already_passed() {
        let b = make_birthday(1990, 1, 15);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(
            b.next_birthday_from(today),
            NaiveDate::from_ymd_opt(2027, 1, 15).unwrap()
        );
    }

    #[test]
    fn next_birthday_is_today() {
        let b = make_birthday(1990, 6, 1);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.next_birthday_from(today), today);
    }

    #[test]
    fn next_birthday_leap_day_in_non_leap_year() {
        let b = make_birthday(2000, 2, 29);
        // 2027 is not a leap year; next year with Feb 29 is 2028
        let today = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert_eq!(
            b.next_birthday_from(today),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
        );
    }

    #[test]
    fn next_birthday_leap_day_in_leap_year() {
        let b = make_birthday(2000, 2, 29);
        // 2028 is a leap year
        let today = NaiveDate::from_ymd_opt(2028, 1, 1).unwrap();
        assert_eq!(
            b.next_birthday_from(today),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
        );
    }

    #[test]
    fn days_until_next_birthday() {
        let b = make_birthday(1990, 6, 10);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.days_until_next_from(today), 9);
    }

    #[test]
    fn days_until_next_is_zero_on_birthday() {
        let b = make_birthday(1990, 6, 1);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.days_until_next_from(today), 0);
    }

    #[test]
    fn turning_age_next_birthday() {
        let b = make_birthday(1990, 12, 25);
        let today = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        assert_eq!(b.turning_age_on(today), 36);
    }
}
