use tokio::process::Command;

use super::error::AppError;

pub struct Injector {
    last_partial: String,
    partial_allowed: bool,
}

impl Injector {
    pub fn new() -> Self {
        Self {
            last_partial: String::new(),
            partial_allowed: true,
        }
    }

    pub fn reset_session(&mut self) {
        self.last_partial.clear();
        self.partial_allowed = true;
    }

    pub fn can_partial(&self) -> bool {
        self.partial_allowed
    }

    pub async fn type_final(&mut self, text: &str) -> Result<(), AppError> {
        self.type_text(text).await?;
        self.last_partial.clear();
        Ok(())
    }

    pub async fn type_partial_replace(&mut self, text: &str) -> Result<(), AppError> {
        if !self.partial_allowed {
            return Ok(());
        }

        if !self.last_partial.is_empty() {
            // Best-effort replacement by backspacing previous partial.
            for _ in self.last_partial.chars() {
                self.press_key("BackSpace").await?;
            }
        }

        if let Err(err) = self.type_text(text).await {
            self.partial_allowed = false;
            return Err(err);
        }

        self.last_partial = text.to_string();
        Ok(())
    }

    async fn press_key(&self, key: &str) -> Result<(), AppError> {
        let output = Command::new("wtype")
            .arg("-k")
            .arg(key)
            .output()
            .await
            .map_err(|e| AppError::new("wtype の実行に失敗しました", e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::new(
                "キー注入に失敗しました",
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    async fn type_text(&self, text: &str) -> Result<(), AppError> {
        let output = Command::new("wtype")
            .arg(text)
            .output()
            .await
            .map_err(|e| AppError::new("wtype の実行に失敗しました", e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::new(
                "テキスト注入に失敗しました",
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}
