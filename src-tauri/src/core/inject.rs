use tokio::process::Command;

use super::error::AppError;

pub struct Injector;

#[derive(Debug, Default)]
pub struct InjectionSession {
    last_partial: String,
    partial_allowed: bool,
}

impl InjectionSession {
    pub fn new() -> Self {
        Self {
            last_partial: String::new(),
            partial_allowed: true,
        }
    }

    pub fn reset(&mut self) {
        self.last_partial.clear();
        self.partial_allowed = true;
    }

    pub fn is_partial_allowed(&self) -> bool {
        self.partial_allowed
    }

    pub fn mark_partial_denied(&mut self) {
        self.partial_allowed = false;
    }

    pub fn set_last_partial(&mut self, text: &str) {
        self.last_partial = text.to_string();
    }

    pub fn last_partial(&self) -> &str {
        &self.last_partial
    }
}

impl Injector {
    pub fn new() -> Self {
        Self
    }

    pub async fn clear_partial(&self, session: &mut InjectionSession) -> Result<(), AppError> {
        if session.last_partial().is_empty() {
            return Ok(());
        }
        for _ in session.last_partial().chars() {
            self.press_key("BackSpace").await?;
        }
        session.set_last_partial("");
        Ok(())
    }

    pub async fn type_final(
        &self,
        session: &mut InjectionSession,
        text: &str,
    ) -> Result<(), AppError> {
        self.clear_partial(session).await?;
        self.type_text(text).await?;
        session.reset();
        Ok(())
    }

    pub async fn type_partial_replace(
        &self,
        session: &mut InjectionSession,
        text: &str,
    ) -> Result<(), AppError> {
        if !session.is_partial_allowed() {
            return Ok(());
        }

        self.clear_partial(session).await?;

        if let Err(err) = self.type_text(text).await {
            session.mark_partial_denied();
            return Err(err);
        }

        session.set_last_partial(text);
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
