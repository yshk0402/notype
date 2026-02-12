use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use super::audio::{cleanup_temp_file, RecordingSession};
use super::config::{save_config, AppConfig};
use super::error::AppError;
use super::inject::{InjectionSession, Injector};
use super::state::{ErrorEvent, ModelDownloadEvent, RuntimeState, TranscriptEvent};
use super::stt::SttService;

struct RuntimeInner {
    state: RuntimeState,
    current_text: String,
    recording: Option<RecordingSession>,
    partial_task: Option<tokio::task::JoinHandle<()>>,
    watchdog_task: Option<tokio::task::JoinHandle<()>>,
    last_toggle_at: Option<std::time::Instant>,
}

impl Default for RuntimeInner {
    fn default() -> Self {
        Self {
            state: RuntimeState::Idle,
            current_text: String::new(),
            recording: None,
            partial_task: None,
            watchdog_task: None,
            last_toggle_at: None,
        }
    }
}

struct RecordingUsecase;

impl RecordingUsecase {
    async fn start_session(&self) -> Result<RecordingSession, AppError> {
        RecordingSession::start().await
    }

    async fn stop_session(
        &self,
        recording: &mut RecordingSession,
    ) -> Result<std::path::PathBuf, AppError> {
        recording.stop().await
    }

    fn spawn_partial_task(
        &self,
        app: AppHandle,
        runtime_inner: Arc<Mutex<RuntimeInner>>,
        config: AppConfig,
        injection: Arc<Mutex<InjectionUsecase>>,
        wav_path: std::path::PathBuf,
        started_at: std::time::Instant,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let stt = SttService::new(config.model.clone());
            let mut last_partial = String::new();
            let partial_in_flight = Arc::new(AtomicBool::new(false));
            let mut tick_ms: u64 = 250;

            loop {
                tokio::time::sleep(Duration::from_millis(tick_ms)).await;

                let elapsed = started_at.elapsed().as_millis() as u64;
                if elapsed >= config.max_record_seconds as u64 * 1000 {
                    break;
                }

                let state = runtime_inner.lock().await.state;
                if state != RuntimeState::Recording {
                    break;
                }

                if !config.realtime_enabled {
                    emit_transcript(
                        &app,
                        TranscriptEvent {
                            partial_text: String::new(),
                            final_text: None,
                            state: RuntimeState::Recording,
                            latency_ms: None,
                        },
                    );
                    tick_ms = 400;
                    continue;
                }

                if partial_in_flight.swap(true, Ordering::SeqCst) {
                    emit_transcript(
                        &app,
                        TranscriptEvent {
                            partial_text: String::new(),
                            final_text: None,
                            state: RuntimeState::Recording,
                            latency_ms: None,
                        },
                    );
                    tick_ms = 400;
                    continue;
                }

                let partial_started = std::time::Instant::now();
                let partial = stt.transcribe_partial_hint(elapsed, &wav_path).await;
                partial_in_flight.store(false, Ordering::SeqCst);
                let partial = partial.unwrap_or_default();
                let partial_latency = partial_started.elapsed().as_millis() as u64;
                tick_ms = if partial_latency > 300 { 400 } else { 250 };

                if partial.is_empty() || partial == last_partial {
                    emit_transcript(
                        &app,
                        TranscriptEvent {
                            partial_text: String::new(),
                            final_text: None,
                            state: RuntimeState::Recording,
                            latency_ms: None,
                        },
                    );
                    continue;
                }

                last_partial = partial.clone();
                emit_transcript(
                    &app,
                    TranscriptEvent {
                        partial_text: partial.clone(),
                        final_text: None,
                        state: RuntimeState::Recording,
                        latency_ms: Some(partial_latency),
                    },
                );

                if config.auto_type {
                    let mut inj = injection.lock().await;
                    if let Err(err) = inj.type_partial_replace(&partial).await {
                        if inj.mark_partial_degraded_once() {
                            emit_error(
                                &app,
                                AppError::new(
                                    "部分注入に失敗したため、この録音では最終結果のみ注入します",
                                    err.details,
                                ),
                            );
                        }
                    }
                }
            }
        })
    }

    fn spawn_watchdog_task(
        &self,
        app: AppHandle,
        runtime_inner: Arc<Mutex<RuntimeInner>>,
        timeout: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;

            let mut timed_out_recording = {
                let mut inner = runtime_inner.lock().await;
                if inner.state != RuntimeState::Recording {
                    return;
                }

                if let Some(task) = inner.partial_task.take() {
                    task.abort();
                }

                inner.state = RuntimeState::Idle;
                inner.recording.take()
            };

            if let Some(mut recording) = timed_out_recording.take() {
                if let Ok(wav) = recording.stop().await {
                    cleanup_temp_file(&wav);
                }
            }

            emit_transcript(
                &app,
                TranscriptEvent {
                    partial_text: String::new(),
                    final_text: None,
                    state: RuntimeState::Idle,
                    latency_ms: None,
                },
            );
            emit_error(
                &app,
                AppError::new(
                    "録音が上限時間を超えたため自動停止しました。Alt+X を再押下して再試行できます",
                    "watchdog forced stop",
                ),
            );
        })
    }
}

struct TranscriptionUsecase;

impl TranscriptionUsecase {
    async fn transcribe(
        &self,
        app: &AppHandle,
        config: &AppConfig,
        wav: &std::path::Path,
    ) -> Result<(String, u64), AppError> {
        let stt = SttService::new(config.model.clone());
        tracing::info!("transcription: ensure model started");

        let model_ready = tokio::time::timeout(
            Duration::from_secs(180),
            stt.ensure_model_with_progress(|progress, status, message| {
                emit_model_download(
                    app,
                    ModelDownloadEvent {
                        progress,
                        status: status.to_string(),
                        message: message.to_string(),
                    },
                );
            }),
        )
        .await;

        match model_ready {
            Ok(Ok(())) => {
                tracing::info!("transcription: ensure model done");
            }
            Ok(Err(err)) => return Err(err),
            Err(err) => {
                return Err(AppError::new(
                    "モデル準備がタイムアウトしました。ネットワークまたは容量を確認してください",
                    err.to_string(),
                ))
            }
        }

        tracing::info!("transcription: final transcribe started");
        match tokio::time::timeout(Duration::from_secs(90), stt.transcribe_final(wav)).await {
            Ok(r) => {
                tracing::info!("transcription: final transcribe finished");
                r
            }
            Err(err) => Err(AppError::new(
                "文字起こし処理がタイムアウトしました",
                err.to_string(),
            )),
        }
    }
}

struct InjectionUsecase {
    injector: Injector,
    session: InjectionSession,
    partial_degraded: bool,
}

impl InjectionUsecase {
    fn new() -> Self {
        Self {
            injector: Injector::new(),
            session: InjectionSession::new(),
            partial_degraded: false,
        }
    }

    fn reset_session(&mut self) {
        self.session.reset();
        self.partial_degraded = false;
    }

    fn mark_partial_degraded_once(&mut self) -> bool {
        if self.partial_degraded {
            return false;
        }
        self.partial_degraded = true;
        true
    }

    async fn type_partial_replace(&mut self, text: &str) -> Result<(), AppError> {
        self.injector
            .type_partial_replace(&mut self.session, text)
            .await
    }

    async fn type_final(&mut self, text: &str) -> Result<(), AppError> {
        self.injector.type_final(&mut self.session, text).await
    }
}

pub struct AppRuntime {
    inner: Arc<Mutex<RuntimeInner>>,
    config: Arc<Mutex<AppConfig>>,
    recording: RecordingUsecase,
    transcription: TranscriptionUsecase,
    injection: Arc<Mutex<InjectionUsecase>>,
}

impl AppRuntime {
    pub fn new(config: AppConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner::default())),
            config: Arc::new(Mutex::new(config)),
            recording: RecordingUsecase,
            transcription: TranscriptionUsecase,
            injection: Arc::new(Mutex::new(InjectionUsecase::new())),
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

    pub async fn toggle_recording(&self, app: AppHandle) -> Result<RuntimeState, AppError> {
        let state = {
            let mut inner = self.inner.lock().await;
            let now = std::time::Instant::now();
            if let Some(last) = inner.last_toggle_at {
                if now.duration_since(last) < Duration::from_millis(450) {
                    tracing::info!("toggle throttled; state={:?}", inner.state);
                    return Ok(inner.state);
                }
            }
            inner.last_toggle_at = Some(now);
            inner.state
        };

        tracing::info!("toggle request received; state={:?}", state);
        if state == RuntimeState::Processing {
            tracing::info!("toggle ignored while processing");
            return Ok(RuntimeState::Processing);
        }

        if state == RuntimeState::Recording {
            tracing::info!("toggle action=stop");
            match self.stop_recording(app).await {
                Ok(_) => {
                    tracing::info!("toggle action=stop done");
                    return Ok(RuntimeState::Ready);
                }
                Err(err) => {
                    tracing::warn!("toggle action=stop failed: {}", err.details);
                    return Err(err);
                }
            }
        }

        tracing::info!("toggle action=start");
        match self.start_recording(app).await {
            Ok(_) => {
                tracing::info!("toggle action=start done");
                Ok(RuntimeState::Recording)
            }
            Err(err) => {
                tracing::warn!("toggle action=start failed: {}", err.details);
                Err(err)
            }
        }
    }

    pub async fn start_recording(&self, app: AppHandle) -> Result<(), AppError> {
        {
            let inner = self.inner.lock().await;
            if inner.state == RuntimeState::Recording {
                return Ok(());
            }
            if inner.state == RuntimeState::Processing {
                return Err(AppError::new(
                    "まだ前回の処理中です。完了後に Alt+X を押してください",
                    "cannot start while processing",
                ));
            }
        }

        self.injection.lock().await.reset_session();

        let recording = self.recording.start_session().await?;
        let start_path = recording.audio_path.clone();
        let started_at = recording.started_at;

        {
            let mut inner = self.inner.lock().await;
            if let Some(task) = inner.watchdog_task.take() {
                task.abort();
            }
            inner.recording = Some(recording);
            inner.state = RuntimeState::Recording;
        }

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
        let partial_task = self.recording.spawn_partial_task(
            app.clone(),
            self.inner.clone(),
            config.clone(),
            self.injection.clone(),
            start_path,
            started_at,
        );
        let watchdog_task = self.recording.spawn_watchdog_task(
            app,
            self.inner.clone(),
            Duration::from_secs(config.max_record_seconds as u64 + 2),
        );

        let mut inner = self.inner.lock().await;
        inner.partial_task = Some(partial_task);
        inner.watchdog_task = Some(watchdog_task);
        tracing::info!("recording started");
        Ok(())
    }

    pub async fn stop_recording(&self, app: AppHandle) -> Result<String, AppError> {
        let maybe_recording = {
            let mut inner = self.inner.lock().await;
            if inner.state != RuntimeState::Recording {
                return Ok(inner.current_text.clone());
            }

            if let Some(task) = inner.partial_task.take() {
                task.abort();
            }
            if let Some(task) = inner.watchdog_task.take() {
                task.abort();
            }

            inner.state = RuntimeState::Processing;
            inner.recording.take()
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

        let stop_result: Result<String, AppError> = async {
            let mut recording = maybe_recording
                .ok_or_else(|| AppError::new("録音セッションが見つかりません", "missing recording"))?;

            tracing::info!("stop_recording: stopping audio session");
            let wav = tokio::time::timeout(
                Duration::from_secs(3),
                self.recording.stop_session(&mut recording),
            )
            .await
            .map_err(|_| {
                AppError::new(
                    "録音停止がタイムアウトしました。Alt+X で再試行してください",
                    "stop_session timeout",
                )
            })??;

            tracing::info!("stop_recording: audio session stopped");
            let config = self.config.lock().await.clone();
            tracing::info!("stop_recording: transcription started");
            let result = self.transcription.transcribe(&app, &config, &wav).await;
            cleanup_temp_file(&wav);
            let (mut text, latency) = result?;
            tracing::info!("stop_recording: transcription done");

            if config.text_cleanup {
                text = cleanup_text(text);
            }

            {
                let mut inner = self.inner.lock().await;
                inner.current_text = text.clone();
                inner.state = RuntimeState::Ready;
            }

            if config.auto_type {
                tokio::time::sleep(Duration::from_millis(120)).await;
                if let Err(err) = self.injection.lock().await.type_final(&text).await {
                    emit_error(
                        &app,
                        AppError::new(
                            "自動入力に失敗しました。フォーカス先を確認して Alt+X で再試行してください",
                            err.details,
                        ),
                    );
                }
            }

            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
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
            tracing::info!("recording stopped");

            Ok(text)
        }
        .await;

        match stop_result {
            Ok(text) => Ok(text),
            Err(err) => {
                self.reset_idle(&app).await;
                emit_error(&app, err.clone());
                tracing::warn!("recording stop failed: {}", err.details);
                Err(err)
            }
        }
    }

    pub async fn type_text(&self, text: String) -> Result<(), AppError> {
        self.injection.lock().await.type_final(&text).await
    }

    pub async fn current_text(&self) -> String {
        self.inner.lock().await.current_text.clone()
    }

    pub async fn reset_idle(&self, app: &AppHandle) {
        let mut inner = self.inner.lock().await;
        if let Some(task) = inner.partial_task.take() {
            task.abort();
        }
        if let Some(task) = inner.watchdog_task.take() {
            task.abort();
        }
        inner.state = RuntimeState::Idle;
        drop(inner);

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
