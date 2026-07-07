use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub mode: String, // "tcp", "udp", "rtu"
    pub unit_id: u8,
    pub serial: Option<SerialConfig>,
    pub network: Option<NetworkConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadRequest {
    pub function_code: u8,
    pub address: u16,
    pub count: u16,
    pub data_type: String, // "u16", "i16", "u32", "i32", "f32", "f64", "string"
    pub byte_order: String,  // "ab", "ba", "abcd", "badc", "cdab", "dcba"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRequest {
    pub function_code: u8,
    pub address: u16,
    pub values: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    pub ok: bool,
    pub message: String,
    pub address: u16,
    pub count: u16,
    pub data_type: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub ok: bool,
    pub message: String,
    pub address: u16,
    pub count: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterStatus {
    pub connected: bool,
    pub mode: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub found: Vec<u8>,
    pub message: String,
}
