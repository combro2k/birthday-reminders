use super::birthday::Birthday;

/// Policy defining when reminders should be sent
#[derive(Debug, Clone)]
pub struct ReminderPolicy {
    pub days_before: Vec<u32>,
}

impl ReminderPolicy {
    pub fn new(days_before: Vec<u32>) -> Self {
        Self { days_before }
    }
}

/// A reminder that is due to be sent
#[derive(Debug, Clone)]
pub struct PendingReminder {
    pub birthday: Birthday,
    pub days_before: u32,
    pub turning_age: u32,
}

impl PendingReminder {
    pub fn message(&self) -> String {
        if self.days_before == 0 {
            format!(
                "🎂 {} turns {} today!",
                self.birthday.name, self.turning_age
            )
        } else if self.days_before == 1 {
            format!(
                "🎂 {} turns {} tomorrow!",
                self.birthday.name, self.turning_age
            )
        } else {
            format!(
                "🎂 {} turns {} in {} days!",
                self.birthday.name, self.turning_age, self.days_before
            )
        }
    }

    pub fn title(&self) -> String {
        if self.days_before == 0 {
            "Birthday Today!".to_string()
        } else {
            "Upcoming Birthday".to_string()
        }
    }
}
