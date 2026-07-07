use tauri::State;
use tokio_serial::available_ports;

use crate::models::{
    ConnectionConfig, MasterStatus, ReadRequest, ReadResult, ScanResult, WriteRequest, WriteResult,
};
use crate::modbus::{connect, disconnect, get_status, read_registers, write_registers, AppState};

#[tauri::command]
pub async fn get_serial_ports() -> Result<Vec<String>, String> {
    match available_ports() {
        Ok(ports) => Ok(ports.into_iter().map(|p| p.port_name).collect()),
        Err(e) => Err(format!("Failed to list serial ports: {}", e)),
    }
}

#[tauri::command]
pub async fn connect_master(
    state: State<'_, AppState>,
    config: ConnectionConfig,
) -> Result<MasterStatus, String> {
    connect(&state, config).await
}

#[tauri::command]
pub async fn disconnect_master(state: State<'_, AppState>) -> Result<MasterStatus, String> {
    disconnect(&state).await
}

#[tauri::command]
pub async fn read_master(
    state: State<'_, AppState>,
    request: ReadRequest,
) -> Result<ReadResult, String> {
    read_registers(&state, request).await
}

#[tauri::command]
pub async fn write_master(
    state: State<'_, AppState>,
    request: WriteRequest,
) -> Result<WriteResult, String> {
    write_registers(&state, request).await
}

#[tauri::command]
pub async fn scan_slaves(
    state: State<'_, AppState>,
    start: u8,
    end: u8,
) -> Result<ScanResult, String> {
    crate::modbus::scan_slaves(&state, start, end).await
}

#[tauri::command]
pub async fn get_master_status(state: State<'_, AppState>) -> Result<MasterStatus, String> {
    Ok(get_status(&state).await)
}
