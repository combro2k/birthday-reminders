#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Not found")]
    NotFound,

    #[error("Already exists")]
    AlreadyExists,

    #[error("Database error: {0}")]
    Database(String),
}
