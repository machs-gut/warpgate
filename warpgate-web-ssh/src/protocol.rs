use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warpgate_protocol_ssh::RCState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    OpenChannel {
        cols: Option<u32>,
        rows: Option<u32>,
    },
    Input {
        channel_id: Uuid,
        #[serde(with = "warpgate_common::helpers::serde_base64")]
        data: Bytes,
    },
    Resize {
        channel_id: Uuid,
        cols: u32,
        rows: u32,
    },
    CloseChannel {
        channel_id: Uuid,
    },
    AcceptHostKey,
    RejectHostKey,
    StartMetrics,
    StopMetrics,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricsStatus {
    Starting,
    Available,
    Unavailable,
    Disabled,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricsSnapshot {
    pub cpu_percent: Option<f64>,
    pub cpu_count: u32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub load1: f64,
    pub rx_bytes_per_sec: Option<f64>,
    pub tx_bytes_per_sec: Option<f64>,
    pub network_interface: Option<String>,
    pub disk_percent: f64,
    pub uptime_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    ConnectionState {
        state: RCState,
    },
    Output {
        channel_id: Uuid,
        #[serde(with = "warpgate_common::helpers::serde_base64")]
        data: Bytes,
    },
    ChannelOpened {
        channel_id: Uuid,
    },
    ChannelClosed {
        channel_id: Uuid,
    },
    Eof {
        channel_id: Uuid,
    },
    ExitStatus {
        channel_id: Uuid,
        code: u32,
    },
    Error {
        message: String,
    },
    HostKeyUnknown {
        host: String,
        port: u16,
        key_type: String,
        key_base64: String,
    },
    MetricsStatus {
        state: MetricsStatus,
        message: Option<String>,
    },
    MetricsSnapshot {
        snapshot: MetricsSnapshot,
    },
}
