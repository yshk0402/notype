use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use zbus::interface;

use super::app::AppRuntime;
use super::error::AppError;
use super::state::ErrorEvent;

pub const BUS_NAME: &str = "dev.notype.app";
pub const OBJECT_PATH: &str = "/dev/notype/app";
pub const INTERFACE_NAME: &str = "dev.notype.app";

#[derive(Clone)]
pub struct IpcController {
    app: AppHandle,
    runtime: Arc<AppRuntime>,
}

impl IpcController {
    pub fn new(app: AppHandle, runtime: Arc<AppRuntime>) -> Self {
        Self { app, runtime }
    }

    fn show_main_impl(&self) {
        if let Some(win) = self.app.get_webview_window("main") {
            if let Err(err) = win.show() {
                tracing::warn!("ShowMain: failed to show main window: {err}");
            }
            if let Err(err) = win.set_focus() {
                tracing::warn!("ShowMain: failed to focus main window: {err}");
            }
        } else {
            tracing::warn!("ShowMain: main window not found");
        }
    }

    fn show_settings_impl(&self) {
        if let Some(win) = self.app.get_webview_window("settings") {
            if let Err(err) = win.show() {
                tracing::warn!("ShowSettings: failed to show settings window: {err}");
            }
            if let Err(err) = win.set_focus() {
                tracing::warn!("ShowSettings: failed to focus settings window: {err}");
            }
        } else {
            tracing::warn!("ShowSettings: settings window not found");
        }
    }

    fn toggle_recording_impl(&self) {
        tracing::info!("ToggleRecording: request received");
        let runtime = self.runtime.clone();
        let app = self.app.clone();
        tauri::async_runtime::spawn(async move {
            match runtime.toggle_recording(app.clone()).await {
                Ok(state) => {
                    tracing::info!("ToggleRecording: done state={state:?}");
                }
                Err(err) => {
                    let _ = app.emit(
                        "notype://error",
                        ErrorEvent {
                            user_message: err.user_message.clone(),
                            details: err.details.clone(),
                        },
                    );
                    tracing::warn!("ToggleRecording: failed to toggle recording: {err}");
                }
            }
        });
    }

    fn quit_impl(&self) {
        tracing::info!("Quit: exiting application by IPC request");
        self.app.exit(0);
    }
}

pub struct IpcService {
    controller: Arc<Mutex<IpcController>>,
}

impl IpcService {
    pub fn new(controller: IpcController) -> Self {
        Self {
            controller: Arc::new(Mutex::new(controller)),
        }
    }
}

#[interface(name = "dev.notype.app")]
impl IpcService {
    #[zbus(name = "ShowMain")]
    async fn show_main(&self) {
        self.controller.lock().await.show_main_impl();
    }

    #[zbus(name = "ShowSettings")]
    async fn show_settings(&self) {
        self.controller.lock().await.show_settings_impl();
    }

    #[zbus(name = "ToggleRecording")]
    async fn toggle_recording(&self) {
        self.controller.lock().await.toggle_recording_impl();
    }

    #[zbus(name = "Quit")]
    async fn quit(&self) {
        self.controller.lock().await.quit_impl();
    }
}

pub async fn try_call_existing(method: &str) -> Result<bool, AppError> {
    let conn = match zbus::Connection::session().await {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };

    let result = conn
        .call_method(
            Some(BUS_NAME),
            OBJECT_PATH,
            Some(INTERFACE_NAME),
            method,
            &(),
        )
        .await;

    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
