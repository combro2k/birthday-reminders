use std::sync::Arc;

use chrono::NaiveDate;

use crate::birthdays::domain::birthday::Birthday;
use crate::birthdays::domain::repository::{BirthdayRepository, NewBirthday, UpdateBirthday};
use crate::users::domain::user::UserId;

pub struct BirthdayCommandService {
    repo: Arc<dyn BirthdayRepository>,
}

impl BirthdayCommandService {
    pub fn new(repo: Arc<dyn BirthdayRepository>) -> Self {
        Self { repo }
    }

    pub async fn add(
        &self,
        user_id: &UserId,
        name: &str,
        birth_date: NaiveDate,
        email: Option<String>,
        phone_number: Option<String>,
        address: Option<String>,
        postal_code: Option<String>,
        city: Option<String>,
        country: Option<String>,
        notes: Option<String>,
    ) -> anyhow::Result<Birthday> {
        validate_birthday_input(
            name,
            birth_date,
            email.as_deref(),
            phone_number.as_deref(),
            address.as_deref(),
            postal_code.as_deref(),
            city.as_deref(),
            country.as_deref(),
            notes.as_deref(),
        )?;

        let new = NewBirthday {
            user_id: user_id.clone(),
            name: name.to_string(),
            birth_date,
            email,
            phone_number,
            address,
            postal_code,
            city,
            country,
            notes,
        };
        let birthday = self
            .repo
            .create(new)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create birthday: {}", e))?;
        Ok(birthday)
    }

    pub async fn update(
        &self,
        id: uuid::Uuid,
        user_id: &UserId,
        name: Option<String>,
        birth_date: Option<NaiveDate>,
        email: Option<Option<String>>,
        phone_number: Option<Option<String>>,
        address: Option<Option<String>>,
        postal_code: Option<Option<String>>,
        city: Option<Option<String>>,
        country: Option<Option<String>>,
        notes: Option<Option<String>>,
    ) -> anyhow::Result<Birthday> {
        // Validate provided fields
        let effective_name = name.as_deref().unwrap_or("placeholder");
        let effective_date = birth_date.unwrap_or(chrono::Local::now().date_naive());
        let effective_email = email.as_ref().and_then(|n| n.as_deref());
        let effective_phone_number = phone_number.as_ref().and_then(|n| n.as_deref());
        let effective_address = address.as_ref().and_then(|n| n.as_deref());
        let effective_postal_code = postal_code.as_ref().and_then(|n| n.as_deref());
        let effective_city = city.as_ref().and_then(|n| n.as_deref());
        let effective_country = country.as_ref().and_then(|n| n.as_deref());
        let effective_notes = notes.as_ref().and_then(|n| n.as_deref());
        validate_birthday_input(
            effective_name,
            effective_date,
            effective_email,
            effective_phone_number,
            effective_address,
            effective_postal_code,
            effective_city,
            effective_country,
            effective_notes,
        )?;

        // Verify ownership
        let existing = self
            .repo
            .find_by_id(&id.into())
            .await
            .map_err(|e| anyhow::anyhow!("Birthday not found: {}", e))?;

        if existing.user_id != *user_id {
            anyhow::bail!("Not authorized to modify this birthday");
        }

        let update = UpdateBirthday {
            name,
            birth_date,
            email,
            phone_number,
            address,
            postal_code,
            city,
            country,
            notes,
        };
        let birthday = self
            .repo
            .update(&id.into(), update)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update birthday: {}", e))?;
        Ok(birthday)
    }

    pub async fn delete(&self, id: uuid::Uuid, user_id: &UserId) -> anyhow::Result<()> {
        // Verify ownership
        let existing = self
            .repo
            .find_by_id(&id.into())
            .await
            .map_err(|e| anyhow::anyhow!("Birthday not found: {}", e))?;

        if existing.user_id != *user_id {
            anyhow::bail!("Not authorized to delete this birthday");
        }

        self.repo
            .delete(&id.into())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete birthday: {}", e))?;
        Ok(())
    }
}

fn validate_birthday_input(
    name: &str,
    birth_date: NaiveDate,
    email: Option<&str>,
    phone_number: Option<&str>,
    address: Option<&str>,
    postal_code: Option<&str>,
    city: Option<&str>,
    country: Option<&str>,
    notes: Option<&str>,
) -> anyhow::Result<()> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("Name cannot be empty");
    }
    if name.len() > 200 {
        anyhow::bail!("Name cannot exceed 200 characters");
    }

    let today = chrono::Local::now().date_naive();
    if birth_date > today {
        anyhow::bail!("Birth date cannot be in the future");
    }

    if let Some(e) = email {
        if e.len() > 255 {
            anyhow::bail!("Email cannot exceed 255 characters");
        }
        if !e.contains('@') {
            anyhow::bail!("Email must contain an '@' symbol");
        }
    }

    if let Some(phone) = phone_number {
        if phone.len() > 50 {
            anyhow::bail!("Phone number cannot exceed 50 characters");
        }
    }

    if let Some(a) = address {
        if a.len() > 255 {
            anyhow::bail!("Address cannot exceed 255 characters");
        }
    }

    if let Some(p) = postal_code {
        if p.len() > 20 {
            anyhow::bail!("Postal code cannot exceed 20 characters");
        }
    }

    if let Some(c) = city {
        if c.len() > 100 {
            anyhow::bail!("City cannot exceed 100 characters");
        }
    }

    if let Some(c) = country {
        if c.len() > 100 {
            anyhow::bail!("Country cannot exceed 100 characters");
        }
    }

    if let Some(n) = notes {
        if n.len() > 2000 {
            anyhow::bail!("Notes cannot exceed 2000 characters");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(name: &str, date: NaiveDate, notes: Option<&str>) -> anyhow::Result<()> {
        validate_birthday_input(name, date, None, None, None, None, None, None, notes)
    }

    #[test]
    fn validate_rejects_empty_name() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        assert!(validate("", date, None).is_err());
        assert!(validate("   ", date, None).is_err());
    }

    #[test]
    fn validate_rejects_long_name() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let long_name = "a".repeat(201);
        assert!(validate(&long_name, date, None).is_err());
    }

    #[test]
    fn validate_accepts_max_length_name() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let name = "a".repeat(200);
        assert!(validate(&name, date, None).is_ok());
    }

    #[test]
    fn validate_rejects_future_date() {
        let future = chrono::Local::now().date_naive() + chrono::Duration::days(1);
        assert!(validate("Test", future, None).is_err());
    }

    #[test]
    fn validate_accepts_past_date() {
        let past = NaiveDate::from_ymd_opt(1990, 6, 15).unwrap();
        assert!(validate("Test", past, None).is_ok());
    }

    #[test]
    fn validate_rejects_long_notes() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let long_notes = "x".repeat(2001);
        assert!(validate("Test", date, Some(&long_notes)).is_err());
    }

    #[test]
    fn validate_accepts_valid_notes() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        assert!(validate("Test", date, Some("A note")).is_ok());
    }

    #[test]
    fn validate_rejects_long_phone_number() {
        let date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let long_phone = "1".repeat(51);
        assert!(
            validate_birthday_input(
                "Test",
                date,
                None,
                Some(&long_phone),
                None,
                None,
                None,
                None,
                None,
            )
            .is_err()
        );
    }
}
