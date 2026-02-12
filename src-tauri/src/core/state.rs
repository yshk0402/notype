use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeState {
    Idle,
    Recording,
    Processing,
    Ready,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEvent {
    pub partial_text: String,
    pub final_text: Option<String>,
    pub state: RuntimeState,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {
    pub user_message: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadEvent {
    pub progress: u8,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyWarningEvent {
    pub missing: Vec<String>,
    pub install_hint: String,
}
