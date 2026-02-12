use std::path::PathBuf;
use std::time::Instant;

use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};

use super::error::AppError;

pub struct RecordingSession {
    pub audio_path: PathBuf,
    pub started_at: Instant,
    child: Child,
}

impl RecordingSession {
    pub async fn start() -> Result<Self, AppError> {
        let audio_path = std::env::temp_dir().join(format!("notype-{}.wav", uuid::Uuid::new_v4()));

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
            audio_path,
            started_at: Instant::now(),
            child,
        })
    }

    pub async fn stop(&mut self) -> Result<PathBuf, AppError> {
        if let Some(pid) = self.child.id() {
            let _ = Command::new("kill")
                .arg("-INT")
                .arg(pid.to_string())
                .status()
                .await;
        }

        let wait_result = timeout(Duration::from_millis(700), self.child.wait()).await;
        if wait_result.is_err() {
            self.child
                .start_kill()
                .map_err(|e| AppError::new("録音停止に失敗しました", e.to_string()))?;

            let forced_wait = timeout(Duration::from_millis(900), self.child.wait()).await;
            if forced_wait.is_err() {
                tracing::warn!("recording child did not exit after SIGINT/SIGKILL timeout");
                return Err(AppError::new(
                    "録音停止がタイムアウトしました。Alt+X で再試行してください",
                    "recording child wait timeout",
                ));
            }
        }
        Ok(self.audio_path.clone())
    }
}

pub fn cleanup_temp_file(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}
