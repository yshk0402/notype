use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use super::audio::{cleanup_temp_file, RecordingSession};
use super::config::{save_config, AppConfig};
use super::error::AppError;
use super::inject::Injector;
use super::state::{ErrorEvent, ModelDownloadEvent, RuntimeState, TranscriptEvent};
use super::stt::SttService;

struct RuntimeInner {
    state: RuntimeState,
    current_text: String,
    recording: Option<RecordingSession>,
    partial_task: Option<tokio::task::JoinHandle<()>>,
    partial_degraded: bool,
}

impl Default for RuntimeInner {
    fn default() -> Self {
        Self {
            state: RuntimeState::Idle,
            current_text: String::new(),
            recording: None,
            partial_task: None,
            partial_degraded: false,
        }
    }
}

pub struct AppRuntime {
    inner: Arc<Mutex<RuntimeInner>>,
    config: Arc<Mutex<AppConfig>>,
    injector: Arc<Mutex<Injector>>,
}

impl AppRuntime {
    pub fn new(config: AppConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                state: RuntimeState::Idle,
                ..Default::default()
            })),
            config: Arc::new(Mutex::new(config)),
            injector: Arc::new(Mutex::new(Injector::new())),
        }
    }

    pub async fn get_config(&self) -> AppConfig {
        self.config.lock().await.clone()
    }

    pub async fn update_config(&self, cfg: AppConfig) -> Result<(), AppError> {
        let current = self.config.lock().await.clone();
        let mut merged = cfg;
        if merged.pill_position.is_none() {
            merged.pill_position = current.pill_position;
        }
        save_config(&merged).map_err(AppError::from)?;
        *self.config.lock().await = merged;
        Ok(())
    }

    pub async fn state(&self) -> RuntimeState {
        self.inner.lock().await.state
    }

    pub async fn start_recording(&self, app: AppHandle) -> Result<(), AppError> {
        let mut inner = self.inner.lock().await;
        if inner.state == RuntimeState::Recording {
            return Ok(());
        }

        let mut injector = self.injector.lock().await;
        injector.reset_session();
        drop(injector);

        let recording = RecordingSession::start().await?;
        let start_path = recording.audio_path.clone();
        let started_at = recording.started_at;
        inner.recording = Some(recording);
        inner.partial_degraded = false;
        inner.state = RuntimeState::Recording;
        drop(inner);

        emit_transcript(
            &app,
            TranscriptEvent {
                partial_text: String::new(),
                final_text: None,
                state: RuntimeState::Recording,
                latency_ms: None,
            },
        );

        let config = self.config.lock().await.clone();
        let runtime_inner = self.inner.clone();
        let injector = self.injector.clone();
        let handle = app.clone();
        let partial_task = tokio::spawn(async move {
            let stt = SttService::new(config.model.clone());
            let mut tick = tokio::time::interval(Duration::from_millis(500));

            loop {
                tick.tick().await;

                let elapsed = started_at.elapsed().as_millis() as u64;
                if elapsed >= config.max_record_seconds as u64 * 1000 {
                    break;
                }

                let state = runtime_inner.lock().await.state;
                if state != RuntimeState::Recording {
                    break;
                }

                let partial = stt
                    .transcribe_partial_hint(elapsed, &start_path)
                    .await
                    .unwrap_or_default();

                if !partial.is_empty() {
                    emit_transcript(
                        &handle,
                        TranscriptEvent {
                            partial_text: partial.clone(),
                            final_text: None,
                            state: RuntimeState::Recording,
                            latency_ms: Some(500),
                        },
                    );

                    if config.auto_type {
                        let mut inj = injector.lock().await;
                        if let Err(err) = inj.type_partial_replace(&partial).await {
                            let mut inner = runtime_inner.lock().await;
                            if !inner.partial_degraded {
                                inner.partial_degraded = true;
                                emit_error(
                                    &handle,
                                    AppError::new(
                                        "部分注入に失敗したため、この録音では最終結果のみ注入します",
                                        err.details,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        });

        self.inner.lock().await.partial_task = Some(partial_task);
        Ok(())
    }

    pub async fn stop_recording(&self, app: AppHandle) -> Result<String, AppError> {
        let mut inner = self.inner.lock().await;
        if inner.state != RuntimeState::Recording {
            return Ok(inner.current_text.clone());
        }

        if let Some(task) = inner.partial_task.take() {
            task.abort();
        }

        inner.state = RuntimeState::Processing;
        let maybe_recording = inner.recording.take();
        drop(inner);

        let mut recording = match maybe_recording {
            Some(session) => session,
            None => {
                self.reset_idle(&app).await;
                let err = AppError::new("録音セッションが見つかりません", "missing recording");
                emit_error(&app, err.clone());
                return Err(err);
            }
        };

        emit_transcript(
            &app,
            TranscriptEvent {
                partial_text: String::new(),
                final_text: None,
                state: RuntimeState::Processing,
                latency_ms: None,
            },
        );

        let wav = match recording.stop().await {
            Ok(path) => path,
            Err(err) => {
                self.reset_idle(&app).await;
                emit_error(&app, err.clone());
                return Err(err);
            }
        };
        let config = self.config.lock().await.clone();
        let stt = SttService::new(config.model.clone());

        if let Err(err) = stt
            .ensure_model_with_progress(|progress, status, message| {
                emit_model_download(
                    &app,
                    ModelDownloadEvent {
                        progress,
                        status: status.to_string(),
                        message: message.to_string(),
                    },
                );
            })
            .await
        {
            self.reset_idle(&app).await;
            emit_error(&app, err.clone());
            return Err(err);
        }

        let result = stt.transcribe_final(&wav).await;
        cleanup_temp_file(&wav);

        match result {
            Ok((mut text, latency)) => {
                if config.text_cleanup {
                    text = cleanup_text(text);
                }

                let mut inner = self.inner.lock().await;
                inner.current_text = text.clone();
                inner.state = RuntimeState::Ready;
                let degraded = inner.partial_degraded;
                drop(inner);

                if config.auto_type {
                    let mut injector = self.injector.lock().await;
                    let auto_type_result = if degraded {
                        injector.type_final(&text).await
                    } else {
                        // In replace mode final should overwrite previous partial preview.
                        if let Err(err) = injector.type_partial_replace("").await {
                            Err(err)
                        } else {
                            injector.type_final(&text).await
                        }
                    };

                    if let Err(err) = auto_type_result {
                        emit_error(
                            &app,
                            AppError::new(
                                "自動入力に失敗しました。Copy で貼り付けできます",
                                err.details,
                            ),
                        );
                    }
                }

                emit_transcript(
                    &app,
                    TranscriptEvent {
                        partial_text: String::new(),
                        final_text: Some(text.clone()),
                        state: RuntimeState::Ready,
                        latency_ms: Some(latency),
                    },
                );
                Ok(text)
            }
            Err(err) => {
                self.reset_idle(&app).await;
                emit_error(&app, err.clone());
                Err(err)
            }
        }
    }

    pub async fn type_text(&self, text: String) -> Result<(), AppError> {
        self.injector.lock().await.type_final(&text).await
    }

    pub async fn current_text(&self) -> String {
        self.inner.lock().await.current_text.clone()
    }

    pub async fn reset_idle(&self, app: &AppHandle) {
        self.inner.lock().await.state = RuntimeState::Idle;
        emit_transcript(
            app,
            TranscriptEvent {
                partial_text: String::new(),
                final_text: None,
                state: RuntimeState::Idle,
                latency_ms: None,
            },
        );
    }
}

fn cleanup_text(input: String) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn emit_transcript(app: &AppHandle, event: TranscriptEvent) {
    let _ = app.emit("notype://transcript", event);
}

fn emit_error(app: &AppHandle, err: AppError) {
    let _ = app.emit(
        "notype://error",
        ErrorEvent {
            user_message: err.user_message,
            details: err.details,
        },
    );
}

fn emit_model_download(app: &AppHandle, event: ModelDownloadEvent) {
    let _ = app.emit("notype://model-download", event);
}
