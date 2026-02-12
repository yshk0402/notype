use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct AppError {
    pub user_message: String,
    pub details: String,
}

impl AppError {
    pub fn new(user_message: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            details: details.into(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.user_message, self.details)
    }
}

impl std::error::Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::new("処理に失敗しました", value.to_string())
    }
}
