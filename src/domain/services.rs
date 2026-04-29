use chrono::NaiveDate;

use super::birthday::Birthday;
use super::reminder::{PendingReminder, ReminderPolicy};

/// Pure domain logic: compute which reminders are due for a set of birthdays
pub fn compute_due_reminders(
    birthdays: &[Birthday],
    policy: &ReminderPolicy,
    today: NaiveDate,
) -> Vec<PendingReminder> {
    let mut due = Vec::new();

    for birthday in birthdays {
        let days_until = birthday.days_until_next_from(today);
        let turning_age = birthday.turning_age_on(today);
        let next_birthday_date = birthday.next_birthday_from(today);

        for &days_before in &policy.days_before {
            if days_until == days_before as i64 {
                due.push(PendingReminder {
                    birthday: birthday.clone(),
                    days_before,
                    next_birthday_date,
                    turning_age,
                });
            }
        }
    }

    due
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::birthday::BirthdayId;
    use crate::domain::user::UserId;

    fn make_birthday(month: u32, day: u32) -> Birthday {
        Birthday {
            id: BirthdayId::new(),
            user_id: UserId::new(),
            name: "Test Person".to_string(),
            birth_date: NaiveDate::from_ymd_opt(1990, month, day).unwrap(),
            notes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_birthday_today() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 15).unwrap();
        let birthday = make_birthday(5, 15);
        let policy = ReminderPolicy::new(vec![0]);

        let due = compute_due_reminders(&[birthday], &policy, today);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].days_before, 0);
        assert_eq!(due[0].turning_age, 36);
    }

    #[test]
    fn test_birthday_in_3_days() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let birthday = make_birthday(5, 15);
        let policy = ReminderPolicy::new(vec![7, 3, 1, 0]);

        let due = compute_due_reminders(&[birthday], &policy, today);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].days_before, 3);
    }

    #[test]
    fn test_no_match() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let birthday = make_birthday(5, 15);
        let policy = ReminderPolicy::new(vec![7, 3, 1, 0]);

        let due = compute_due_reminders(&[birthday], &policy, today);
        assert!(due.is_empty());
    }
}
