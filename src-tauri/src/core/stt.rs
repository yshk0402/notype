use std::path::{Path, PathBuf};
use std::time::Instant;

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::config::ModelSize;
use super::error::AppError;

pub struct SttService {
    pub model: ModelSize,
    pub model_dir: PathBuf,
}

impl SttService {
    pub fn new(model: ModelSize) -> Self {
        let model_dir = std::env::var("NOTYPE_MODEL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("notype-models"));

        Self { model, model_dir }
    }

    pub fn model_filename(&self) -> &'static str {
        match self.model {
            ModelSize::Small => "ggml-small.bin",
            ModelSize::Medium => "ggml-medium.bin",
        }
    }

    pub fn model_path(&self) -> PathBuf {
        self.model_dir.join(self.model_filename())
    }

    pub async fn ensure_model(&self) -> Result<(), AppError> {
        self.ensure_model_with_progress(|_, _, _| {}).await
    }

    pub async fn ensure_model_with_progress<F>(&self, mut progress: F) -> Result<(), AppError>
    where
        F: FnMut(u8, &str, &str),
    {
        std::fs::create_dir_all(&self.model_dir)
            .map_err(|e| AppError::new("モデル保存先の準備に失敗しました", e.to_string()))?;

        if self.model_path().exists() {
            progress(100, "ready", "モデルは既に利用可能です");
            return Ok(());
        }

        progress(0, "downloading", "初回モデルをダウンロードしています");
        let url = model_download_url(self.model);
        let response = reqwest::get(url).await.map_err(|e| {
            AppError::new(
                "モデルのダウンロードに失敗しました",
                format!("request failed: {e}"),
            )
        })?;

        if !response.status().is_success() {
            return Err(AppError::new(
                "モデルのダウンロードに失敗しました",
                format!("unexpected status: {}", response.status()),
            ));
        }

        let total_size = response.content_length();
        let target = self.model_path();
        let part = target.with_extension("bin.part");
        let mut file = tokio::fs::File::create(&part).await.map_err(|e| {
            AppError::new(
                "モデル保存ファイルの作成に失敗しました",
                format!("create {}: {e}", part.display()),
            )
        })?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                AppError::new(
                    "モデルのダウンロードに失敗しました",
                    format!("stream failed: {e}"),
                )
            })?;
            file.write_all(&chunk).await.map_err(|e| {
                AppError::new("モデル保存に失敗しました", format!("write failed: {e}"))
            })?;

            downloaded += chunk.len() as u64;
            if let Some(total) = total_size {
                if total > 0 {
                    let pct = ((downloaded as f64 / total as f64) * 100.0).round() as u8;
                    progress(pct.min(99), "downloading", "モデルを取得中です");
                }
            }
        }

        file.flush()
            .await
            .map_err(|e| AppError::new("モデル保存に失敗しました", format!("flush failed: {e}")))?;

        tokio::fs::rename(&part, &target).await.map_err(|e| {
            AppError::new(
                "モデル保存に失敗しました",
                format!("rename {} -> {}: {e}", part.display(), target.display()),
            )
        })?;

        progress(100, "ready", "モデル準備が完了しました");
        Ok(())
    }

    pub async fn transcribe_final(&self, wav_path: &Path) -> Result<(String, u64), AppError> {
        self.ensure_model().await?;
        let started = Instant::now();
        let txt_path = PathBuf::from(format!("{}.txt", wav_path.display()));
        let _ = std::fs::remove_file(&txt_path);

        let output = Command::new("whisper-cli")
            .arg("-m")
            .arg(self.model_path())
            .arg("-f")
            .arg(wav_path)
            .arg("-otxt")
            .arg("-nt")
            .arg("-l")
            .arg("ja")
            .output()
            .await
            .map_err(|e| {
                AppError::new(
                    "文字起こし実行に失敗しました。whisper-cli を確認してください",
                    e.to_string(),
                )
            })?;

        if !output.status.success() {
            return Err(AppError::new(
                "文字起こしに失敗しました",
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        // whisper-cli with -otxt writes to "<audio_path>.txt". Prefer this file.
        let mut text = std::fs::read_to_string(&txt_path)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if text.is_empty() {
            text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        let _ = std::fs::remove_file(&txt_path);
        Ok((text, started.elapsed().as_millis() as u64))
    }

    pub async fn transcribe_partial_hint(
        &self,
        elapsed_ms: u64,
        wav_path: &Path,
    ) -> Result<String, AppError> {
        if !wav_path.exists() {
            return Ok(String::new());
        }

        let seconds = elapsed_ms / 1000;
        Ok(format!("…録音中 {}s", seconds))
    }
}

fn model_download_url(model: ModelSize) -> &'static str {
    match model {
        ModelSize::Small => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        }
        ModelSize::Medium => {
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_filename_matches_size() {
        let small = SttService::new(ModelSize::Small);
        let medium = SttService::new(ModelSize::Medium);
        assert_eq!(small.model_filename(), "ggml-small.bin");
        assert_eq!(medium.model_filename(), "ggml-medium.bin");
    }

    #[test]
    fn model_download_url_is_defined() {
        assert!(model_download_url(ModelSize::Small).contains("ggml-small.bin"));
        assert!(model_download_url(ModelSize::Medium).contains("ggml-medium.bin"));
    }
}
