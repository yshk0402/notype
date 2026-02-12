use std::path::PathBuf;
use std::time::Instant;

use tokio::process::{Child, Command};
use uuid::Uuid;

use super::error::AppError;

pub struct RecordingSession {
    pub id: Uuid,
    pub audio_path: PathBuf,
    pub started_at: Instant,
    child: Child,
}

impl RecordingSession {
    pub async fn start() -> Result<Self, AppError> {
        let id = Uuid::new_v4();
        let audio_path = std::env::temp_dir().join(format!("notype-{}.wav", id));

        let child = Command::new("arecord")
            .arg("-q")
            .arg("-f")
            .arg("S16_LE")
            .arg("-r")
            .arg("16000")
            .arg("-c")
            .arg("1")
            .arg(&audio_path)
            .spawn()
            .map_err(|e| {
                AppError::new(
                    "録音開始に失敗しました。arecord が使えるか確認してください",
                    e.to_string(),
                )
            })?;

        Ok(Self {
            id,
            audio_path,
            started_at: Instant::now(),
            child,
        })
    }

    pub async fn stop(&mut self) -> Result<PathBuf, AppError> {
        self.child
            .start_kill()
            .map_err(|e| AppError::new("録音停止に失敗しました", e.to_string()))?;
        let _ = self.child.wait().await;
        Ok(self.audio_path.clone())
    }
}

pub fn cleanup_temp_file(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}
