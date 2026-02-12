#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;

use std::sync::Arc;
use std::{env, path::Path};

use core::app::AppRuntime;
use core::config::{load_config, AppConfig, PillPosition};
use core::error::AppError;
use core::ipc::{try_call_existing, IpcController, IpcService, BUS_NAME, OBJECT_PATH};
use core::state::{DependencyWarningEvent, RuntimeState};
use tauri::Manager;
use tauri::{Emitter, PhysicalPosition, Position, WindowEvent};
use tokio::process::Command;

#[derive(Clone)]
pub(crate) struct SharedRuntime(pub Arc<AppRuntime>);

#[derive(Clone)]
struct StartupFlags {
    show_settings: bool,
    initial_pill_position: Option<PillPosition>,
    toggle_on_startup: bool,
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, SharedRuntime>) -> Result<AppConfig, String> {
    Ok(state.0.get_config().await)
}

#[tauri::command]
async fn update_config(
    state: tauri::State<'_, SharedRuntime>,
    cfg: AppConfig,
) -> Result<(), String> {
    state.0.update_config(cfg).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_pill_position(
    state: tauri::State<'_, SharedRuntime>,
) -> Result<Option<PillPosition>, String> {
    Ok(state.0.get_config().await.pill_position)
}

#[tauri::command]
async fn set_pill_position(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedRuntime>,
    position: PillPosition,
) -> Result<(), String> {
    let mut cfg = state.0.get_config().await;
    cfg.pill_position = Some(position);
    state
        .0
        .update_config(cfg)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(main) = app.get_webview_window("main") {
        main.set_position(Position::Physical(PhysicalPosition::new(
            position.x, position.y,
        )))
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn start_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedRuntime>,
) -> Result<(), String> {
    state
        .0
        .start_recording(app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedRuntime>,
) -> Result<String, String> {
    state.0.stop_recording(app).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedRuntime>,
) -> Result<RuntimeState, String> {
    state
        .0
        .toggle_recording(app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn prepare_injection_target(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main) = app.get_webview_window("main") {
        main.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn type_text(state: tauri::State<'_, SharedRuntime>, text: String) -> Result<(), String> {
    state.0.type_text(text).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn copy_text(text: String) -> Result<(), String> {
    let output = Command::new("wl-copy")
        .arg(text)
        .output()
        .await
        .map_err(|e| AppError::new("コピーに失敗しました", e.to_string()).to_string())?;
    if !output.status.success() {
        return Err(AppError::new(
            "コピーに失敗しました",
            String::from_utf8_lossy(&output.stderr).to_string(),
        )
        .to_string());
    }
    Ok(())
}

#[tauri::command]
async fn current_text(state: tauri::State<'_, SharedRuntime>) -> Result<String, String> {
    Ok(state.0.current_text().await)
}

#[tauri::command]
async fn get_runtime_state(state: tauri::State<'_, SharedRuntime>) -> Result<RuntimeState, String> {
    Ok(state.0.state().await)
}

#[tauri::command]
async fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(settings) = app.get_webview_window("settings") {
        let _ = settings.unminimize();
        let _ = settings.set_always_on_top(true);
        settings.show().map_err(|e| e.to_string())?;
        settings.set_focus().map_err(|e| e.to_string())?;
        let _ = settings.set_visible_on_all_workspaces(true);
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
            let _ = settings.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
fn check_runtime_dependencies(app: tauri::AppHandle) -> Vec<String> {
    let required = [
        ("arecord", "sudo apt-get install -y alsa-utils"),
        ("wtype", "sudo apt-get install -y wtype"),
        ("wl-copy", "sudo apt-get install -y wl-clipboard"),
        ("whisper-cli", "./scripts/install-whisper-cli-local.sh"),
        ("sxhkd", "sudo apt-get install -y sxhkd"),
    ];
    let missing: Vec<String> = required
        .iter()
        .filter(|(name, _)| !command_exists(name))
        .map(|(name, install)| format!("{name} (install: {install})"))
        .collect();

    if !missing.is_empty() {
        let _ = app.emit(
            "notype://dependency-warning",
            DependencyWarningEvent {
                missing: missing.clone(),
                install_hint: "sudo apt-get install -y alsa-utils wtype wl-clipboard sxhkd && ./scripts/install-whisper-cli-local.sh".to_string(),
            },
        );
    }

    missing
}

#[tokio::main]
async fn main() {
    init_tracing();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let has_toggle = args.iter().any(|a| a == "--toggle");

    if !args.is_empty() {
        if args.iter().any(|a| a == "--quit") {
            let called = try_call_existing("Quit").await.unwrap_or(false);
            if called {
                return;
            }
        }

        if args.iter().any(|a| a == "--settings") {
            let called = try_call_existing("ShowSettings").await.unwrap_or(false);
            if called {
                return;
            }
        }

        if has_toggle {
            tracing::info!("toggle requested from cli");
            let called = try_call_existing("ToggleRecording").await.unwrap_or(false);
            if called {
                tracing::info!("toggle routed to existing instance");
                return;
            }
            tracing::info!("no existing instance; will start and toggle on startup");
        }
    } else {
        let called = try_call_existing("ShowMain").await.unwrap_or(false);
        if called {
            return;
        }
    }

    let config = load_config().unwrap_or_default();
    let initial_pill_position = config.pill_position;
    let runtime = SharedRuntime(Arc::new(AppRuntime::new(config)));
    let flags = StartupFlags {
        show_settings: args.iter().any(|a| a == "--settings"),
        initial_pill_position,
        toggle_on_startup: has_toggle,
    };

    tauri::Builder::default()
        .manage(runtime)
        .manage(std::sync::Mutex::new(flags))
        .invoke_handler(tauri::generate_handler![
            get_config,
            update_config,
            get_pill_position,
            set_pill_position,
            start_recording,
            stop_recording,
            toggle_recording,
            prepare_injection_target,
            type_text,
            copy_text,
            current_text,
            get_runtime_state,
            show_settings,
            check_runtime_dependencies
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let shared_runtime = app.state::<SharedRuntime>().0.clone();
            tauri::async_runtime::spawn(async move {
                let controller = IpcController::new(app_handle.clone(), shared_runtime);
                let service = IpcService::new(controller);

                let conn = zbus::connection::Builder::session()
                    .expect("session bus")
                    .name(BUS_NAME)
                    .expect("bus name")
                    .serve_at(OBJECT_PATH, service)
                    .expect("serve object")
                    .build()
                    .await
                    .expect("dbus build");

                let _keepalive = conn;
                std::future::pending::<()>().await;
            });

            let state = app.state::<std::sync::Mutex<StartupFlags>>();
            let (show_settings, initial_pill_position, toggle_on_startup) = state
                .lock()
                .map(|s| {
                    (
                        s.show_settings,
                        s.initial_pill_position,
                        s.toggle_on_startup,
                    )
                })
                .unwrap_or((false, None, false));

            if show_settings {
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_always_on_top(true);
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });

                if let Some(pos) = initial_pill_position {
                    let _ =
                        win.set_position(Position::Physical(PhysicalPosition::new(pos.x, pos.y)));
                }
            }

            if toggle_on_startup {
                let app_handle = app.handle().clone();
                let runtime = app.state::<SharedRuntime>().0.clone();
                tauri::async_runtime::spawn(async move {
                    tracing::info!("toggle_on_startup: starting recording");
                    let _ = runtime.start_recording(app_handle).await;
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("notype=info,tauri=info")),
        )
        .try_init();
}

fn command_exists(name: &str) -> bool {
    let Some(path_env) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path_env).any(|dir| {
        let bin = Path::new(&dir).join(name);
        bin.is_file()
    })
}
