use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{timeout, Duration};
use tokio_modbus::prelude::*;
use tokio_serial::SerialStream;

use crate::models::{
    ConnectionConfig, LogEvent, MasterStatus, ReadRequest, ReadResult, ScanResult, SerialConfig,
    StatusEvent, WriteRequest, WriteResult,
};

pub struct AppState {
    pub context: Mutex<Option<ClientContext>>,
    pub app_handle: Mutex<Option<AppHandle>>,
    pub status: RwLock<MasterStatus>,
}

pub enum ClientContext {
    Tcp(tokio_modbus::client::Context),
    Rtu(tokio_modbus::client::Context),
    Udp(Arc<UdpSocket>, String, u8),
}

fn parse_serial_config(serial: &SerialConfig) -> Result<tokio_serial::SerialStream, String> {
    let data_bits = match serial.data_bits {
        5 => tokio_serial::DataBits::Five,
        6 => tokio_serial::DataBits::Six,
        7 => tokio_serial::DataBits::Seven,
        _ => tokio_serial::DataBits::Eight,
    };
    let parity = match serial.parity.as_str() {
        "Even" => tokio_serial::Parity::Even,
        "Odd" => tokio_serial::Parity::Odd,
        _ => tokio_serial::Parity::None,
    };
    let stop_bits = match serial.stop_bits {
        2 => tokio_serial::StopBits::Two,
        _ => tokio_serial::StopBits::One,
    };

    let builder = tokio_serial::new(serial.port.clone(), serial.baud_rate)
        .data_bits(data_bits)
        .parity(parity)
        .stop_bits(stop_bits)
        .flow_control(tokio_serial::FlowControl::None);

    SerialStream::open(&builder).map_err(|e| format!("Serial open error: {}", e))
}

async fn emit_log(app: &Option<AppHandle>, message: String) {
    if let Some(handle) = app {
        let _ = handle.emit("modbus-log", LogEvent { message });
    }
}

async fn emit_status(app: &Option<AppHandle>, message: String) {
    if let Some(handle) = app {
        let _ = handle.emit("modbus-status", StatusEvent { message });
    }
}

pub async fn connect(
    state: &AppState,
    config: ConnectionConfig,
) -> Result<MasterStatus, String> {
    let mut ctx = state.context.lock().await;
    if ctx.is_some() {
        return Ok(MasterStatus {
            connected: true,
            mode: config.mode.clone(),
            details: "Already connected".to_string(),
        });
    }

    let app = state.app_handle.lock().await.clone();
    let mode = config.mode.clone();
    let unit = config.unit_id;

    let new_ctx = match config.mode.as_str() {
        "tcp" => {
            let net = config.network.as_ref().ok_or("Missing network config")?;
            let addr = format!("{}:{}", net.host, net.port)
                .parse()
                .map_err(|e| format!("Invalid address: {}", e))?;
            let c = if unit == 0 {
                tcp::connect(addr).await
            } else {
                tcp::connect_slave(addr, Slave(unit)).await
            }
            .map_err(|e| format!("TCP connect error: {:?}", e))?;
            ClientContext::Tcp(c)
        }
        "rtu" => {
            let serial = config.serial.as_ref().ok_or("Missing serial config")?;
            let port = parse_serial_config(serial)?;
            let c = rtu::attach_slave(port, Slave(unit));
            ClientContext::Rtu(c)
        }
        "udp" => {
            let net = config.network.as_ref().ok_or("Missing network config")?;
            let local: std::net::SocketAddr = "0.0.0.0:0"
                .parse()
                .map_err(|e| format!("Invalid local address: {}", e))?;
            let socket = UdpSocket::bind(local)
                .await
                .map_err(|e| format!("UDP bind error: {}", e))?;
            ClientContext::Udp(Arc::new(socket), format!("{}:{}", net.host, net.port), unit)
        }
        _ => return Err(format!("Unsupported mode: {}", config.mode)),
    };

    emit_status(&app, format!("Connected via {}", mode)).await;
    *ctx = Some(new_ctx);

    let mut status = state.status.write().await;
    *status = MasterStatus {
        connected: true,
        mode: mode.clone(),
        details: "Connected".to_string(),
    };

    Ok(MasterStatus {
        connected: true,
        mode,
        details: "Connected".to_string(),
    })
}

pub async fn disconnect(state: &AppState) -> Result<MasterStatus, String> {
    let mut ctx = state.context.lock().await;
    let app = state.app_handle.lock().await.clone();

    if let Some(c) = ctx.take() {
        match c {
            ClientContext::Tcp(mut c) | ClientContext::Rtu(mut c) => {
                let _ = c.disconnect().await;
            }
            ClientContext::Udp(_, _, _) => {}
        }
        emit_status(&app, "Disconnected".to_string()).await;
    }

    let mut status = state.status.write().await;
    *status = MasterStatus {
        connected: false,
        mode: String::new(),
        details: "Disconnected".to_string(),
    };

    Ok(MasterStatus {
        connected: false,
        mode: String::new(),
        details: "Disconnected".to_string(),
    })
}

async fn tcp_rtu_read(
    ctx: &mut tokio_modbus::client::Context,
    req: &ReadRequest,
) -> Result<Vec<u16>, String> {
    let fc = req.function_code;
    let addr = req.address;
    let count = req.count;

    let result = match fc {
        0x01 => ctx
            .read_coils(addr, count)
            .await
            .map_err(|e| format!("Transport error: {}", e))?
            .map_err(|e| format!("Modbus exception: {:?}", e))?
            .into_iter()
            .map(|b| b as u16)
            .collect(),
        0x02 => ctx
            .read_discrete_inputs(addr, count)
            .await
            .map_err(|e| format!("Transport error: {}", e))?
            .map_err(|e| format!("Modbus exception: {:?}", e))?
            .into_iter()
            .map(|b| b as u16)
            .collect(),
        0x03 => ctx
            .read_holding_registers(addr, count)
            .await
            .map_err(|e| format!("Transport error: {}", e))?
            .map_err(|e| format!("Modbus exception: {:?}", e))?,
        0x04 => ctx
            .read_input_registers(addr, count)
            .await
            .map_err(|e| format!("Transport error: {}", e))?
            .map_err(|e| format!("Modbus exception: {:?}", e))?,
        _ => return Err(format!("Unsupported read function code: {}", fc)),
    };

    Ok(result)
}

async fn udp_transaction(
    socket: &UdpSocket,
    remote: &str,
    unit: u8,
    pdu: &[u8],
) -> Result<Vec<u8>, String> {
    let txn = 1u16;
    let len = (1 + pdu.len()) as u16;
    let mut request = Vec::with_capacity(7 + pdu.len());
    request.extend_from_slice(&txn.to_be_bytes());
    request.extend_from_slice(&0u16.to_be_bytes()); // protocol id
    request.extend_from_slice(&len.to_be_bytes());
    request.push(unit);
    request.extend_from_slice(pdu);

    socket
        .send_to(&request, remote)
        .await
        .map_err(|e| format!("UDP send error: {}", e))?;

    let mut buf = [0u8; 1024];
    let (len, _) = timeout(Duration::from_secs(3), socket.recv_from(&mut buf))
        .await
        .map_err(|_| "UDP receive timeout")?
        .map_err(|e| format!("UDP recv error: {}", e))?;

    let response = &buf[..len];
    if response.len() < 7 {
        return Err("UDP response too short".to_string());
    }
    let resp_len = u16::from_be_bytes([response[4], response[5]]) as usize;
    if response.len() < 7 + resp_len - 1 {
        return Err("UDP response length mismatch".to_string());
    }

    Ok(response[7..7 + resp_len - 1].to_vec())
}

fn build_read_pdu(function_code: u8, address: u16, count: u16) -> Vec<u8> {
    let mut pdu = vec![function_code];
    pdu.extend_from_slice(&address.to_be_bytes());
    pdu.extend_from_slice(&count.to_be_bytes());
    pdu
}

fn build_write_single_pdu(function_code: u8, address: u16, value: u16) -> Vec<u8> {
    let mut pdu = vec![function_code];
    pdu.extend_from_slice(&address.to_be_bytes());
    pdu.extend_from_slice(&value.to_be_bytes());
    pdu
}

fn build_write_multiple_pdu(function_code: u8, address: u16, values: &[u16]) -> Vec<u8> {
    let mut pdu = vec![function_code];
    pdu.extend_from_slice(&address.to_be_bytes());
    pdu.extend_from_slice(&(values.len() as u16).to_be_bytes());
    if function_code == 0x0F {
        let bytes = coils_to_bytes(values);
        pdu.push(bytes.len() as u8);
        pdu.extend_from_slice(&bytes);
    } else {
        pdu.push((values.len() * 2) as u8);
        for v in values {
            pdu.extend_from_slice(&v.to_be_bytes());
        }
    }
    pdu
}

fn coils_to_bytes(values: &[u16]) -> Vec<u8> {
    if values.is_empty() {
        return vec![];
    }
    let mut bytes = vec![0u8; (values.len() + 7) / 8];
    for (i, v) in values.iter().enumerate() {
        if *v != 0 {
            bytes[i / 8] |= 1 << (i % 8);
        }
    }
    bytes
}

async fn udp_read(
    socket: &UdpSocket,
    remote: &str,
    unit: u8,
    req: &ReadRequest,
) -> Result<Vec<u16>, String> {
    let fc = req.function_code;
    let pdu = build_read_pdu(fc, req.address, req.count);
    let resp = udp_transaction(socket, remote, unit, &pdu).await?;

    if resp.len() < 2 {
        return Err("UDP response PDU too short".to_string());
    }
    if resp[0] & 0x80 != 0 {
        return Err(format!("Modbus exception: 0x{:02X}", resp[1]));
    }

    let fc_resp = resp[0];
    if fc_resp != fc {
        return Err(format!("Function code mismatch: expected {}, got {}", fc, fc_resp));
    }

    let data = &resp[2..];
    match fc {
        0x01 | 0x02 => {
            let byte_count = data[0] as usize;
            let bytes = &data[1..1 + byte_count];
            let mut bits = Vec::new();
            for i in 0..req.count as usize {
                let byte_index = i / 8;
                let bit_index = i % 8;
                if byte_index < bytes.len() {
                    bits.push(((bytes[byte_index] >> bit_index) & 1) as u16);
                } else {
                    bits.push(0);
                }
            }
            Ok(bits)
        }
        0x03 | 0x04 => {
            let byte_count = data[0] as usize;
            let bytes = &data[1..1 + byte_count];
            let mut regs = Vec::new();
            for chunk in bytes.chunks(2) {
                if chunk.len() == 2 {
                    regs.push(u16::from_be_bytes([chunk[0], chunk[1]]));
                }
            }
            Ok(regs)
        }
        _ => Err(format!("Unsupported read function code: {}", fc)),
    }
}

pub async fn read_registers(
    state: &AppState,
    req: ReadRequest,
) -> Result<ReadResult, String> {
    let mut ctx_guard = state.context.lock().await;
    let ctx = ctx_guard.as_mut().ok_or("Not connected")?;
    let app = state.app_handle.lock().await.clone();

    let raw = match ctx {
        ClientContext::Tcp(c) | ClientContext::Rtu(c) => {
            timeout(Duration::from_secs(5), tcp_rtu_read(c, &req))
                .await
                .map_err(|_| "Read timeout")?
        }
        ClientContext::Udp(socket, remote, unit) => {
            timeout(Duration::from_secs(5), udp_read(socket, remote, *unit, &req))
                .await
                .map_err(|_| "Read timeout")?
        }
    };

    let raw = match raw {
        Ok(v) => v,
        Err(e) => {
            emit_status(&app, format!("Read failed: {}", e)).await;
            return Ok(ReadResult {
                ok: false,
                message: e,
                address: req.address,
                count: req.count,
                data_type: req.data_type.clone(),
                values: vec![],
            });
        }
    };

    let values = decode_values(&raw, &req.data_type, &req.byte_order);
    let msg = format!("Read FC{} @{} x{}: {:?}", req.function_code, req.address, req.count, values);
    emit_log(&app, msg.clone()).await;
    emit_status(&app, "Read OK".to_string()).await;

    Ok(ReadResult {
        ok: true,
        message: msg,
        address: req.address,
        count: req.count,
        data_type: req.data_type.clone(),
        values,
    })
}

async fn tcp_rtu_write(
    ctx: &mut tokio_modbus::client::Context,
    req: &WriteRequest,
) -> Result<(), String> {
    let fc = req.function_code;
    let addr = req.address;

    match fc {
        0x05 => {
            if req.values.is_empty() {
                return Err("No value provided for write single coil".to_string());
            }
            ctx.write_single_coil(addr, req.values[0] != 0)
                .await
                .map_err(|e| format!("Transport error: {}", e))?
                .map_err(|e| format!("Modbus exception: {:?}", e))?;
        }
        0x06 => {
            if req.values.is_empty() {
                return Err("No value provided for write single register".to_string());
            }
            ctx.write_single_register(addr, req.values[0])
                .await
                .map_err(|e| format!("Transport error: {}", e))?
                .map_err(|e| format!("Modbus exception: {:?}", e))?;
        }
        0x0F => {
            let bools: Vec<bool> = req.values.iter().map(|v| *v != 0).collect();
            ctx.write_multiple_coils(addr, &bools)
                .await
                .map_err(|e| format!("Transport error: {}", e))?
                .map_err(|e| format!("Modbus exception: {:?}", e))?;
        }
        0x10 => {
            ctx.write_multiple_registers(addr, &req.values)
                .await
                .map_err(|e| format!("Transport error: {}", e))?
                .map_err(|e| format!("Modbus exception: {:?}", e))?;
        }
        _ => return Err(format!("Unsupported write function code: {}", fc)),
    }

    Ok(())
}

async fn udp_write(
    socket: &UdpSocket,
    remote: &str,
    unit: u8,
    req: &WriteRequest,
) -> Result<(), String> {
    let fc = req.function_code;
    let pdu = build_write_multiple_pdu(fc, req.address, &req.values);
    let resp = udp_transaction(socket, remote, unit, &pdu).await?;

    if resp.len() < 2 {
        return Err("UDP response PDU too short".to_string());
    }
    if resp[0] & 0x80 != 0 {
        return Err(format!("Modbus exception: 0x{:02X}", resp[1]));
    }

    let fc_resp = resp[0];
    if fc_resp != fc {
        return Err(format!("Function code mismatch: expected {}, got {}", fc, fc_resp));
    }

    Ok(())
}

pub async fn write_registers(
    state: &AppState,
    req: WriteRequest,
) -> Result<WriteResult, String> {
    let mut ctx_guard = state.context.lock().await;
    let ctx = ctx_guard.as_mut().ok_or("Not connected")?;
    let app = state.app_handle.lock().await.clone();

    let result = match ctx {
        ClientContext::Tcp(c) | ClientContext::Rtu(c) => {
            timeout(Duration::from_secs(5), tcp_rtu_write(c, &req))
                .await
                .map_err(|_| "Write timeout")?
        }
        ClientContext::Udp(socket, remote, unit) => {
            timeout(Duration::from_secs(5), udp_write(socket, remote, *unit, &req))
                .await
                .map_err(|_| "Write timeout")?
        }
    };

    let msg = format!(
        "Write FC{} @{} x{}: {:?}",
        req.function_code, req.address, req.values.len(), req.values
    );

    match result {
        Ok(()) => {
            emit_log(&app, msg.clone()).await;
            emit_status(&app, "Write OK".to_string()).await;
            Ok(WriteResult {
                ok: true,
                message: msg,
                address: req.address,
                count: req.values.len() as u16,
            })
        }
        Err(e) => {
            emit_status(&app, format!("Write failed: {}", e)).await;
            Ok(WriteResult {
                ok: false,
                message: e,
                address: req.address,
                count: req.values.len() as u16,
            })
        }
    }
}

pub async fn scan_slaves(
    state: &AppState,
    start: u8,
    end: u8,
) -> Result<ScanResult, String> {
    let mut found = Vec::new();
    let mut ctx_guard = state.context.lock().await;
    let ctx = ctx_guard.as_mut().ok_or("Not connected")?;

    // For scanning, attempt a read holding registers at address 0 count 1 for each slave ID
    // This is a basic scan; TCP mode needs connect_slave per ID, UDP needs per-ID request.
    let app = state.app_handle.lock().await.clone();

    emit_status(&app, "Scanning...".to_string()).await;

    for id in start..=end {
        let req = ReadRequest {
            function_code: 0x03,
            address: 0,
            count: 1,
            data_type: "u16".to_string(),
            byte_order: "ab".to_string(),
        };

        let result = match ctx {
            ClientContext::Tcp(c) | ClientContext::Rtu(c) => {
                // For TCP, slave is already set at connect. For RTU, changing slave requires reconnect.
                // Simplified: just try current connection.
                timeout(Duration::from_millis(300), tcp_rtu_read(c, &req)).await
            }
            ClientContext::Udp(socket, remote, _) => {
                timeout(Duration::from_millis(300), udp_read(socket, remote, id, &req)).await
            }
        };

        if let Ok(Ok(_)) = result {
            found.push(id);
        }
    }

    let msg = if found.is_empty() {
        "No slaves found".to_string()
    } else {
        format!("Found slaves: {:?}", found)
    };
    emit_status(&app, msg.clone()).await;

    Ok(ScanResult { found, message: msg })
}

pub async fn get_status(state: &AppState) -> MasterStatus {
    state.status.read().await.clone()
}

fn decode_values(values: &[u16], data_type: &str, byte_order: &str) -> Vec<String> {
    if values.is_empty() {
        return vec![];
    }

    match data_type {
        "u16" => values.iter().map(|v| v.to_string()).collect(),
        "i16" => values
            .iter()
            .map(|v| (*v as i16).to_string())
            .collect(),
        "u32" => values
            .chunks(2)
            .map(|c| {
                let bytes = to_bytes(c, byte_order);
                u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string()
            })
            .collect(),
        "i32" => values
            .chunks(2)
            .map(|c| {
                let bytes = to_bytes(c, byte_order);
                i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string()
            })
            .collect(),
        "f32" => values
            .chunks(2)
            .map(|c| {
                let bytes = to_bytes(c, byte_order);
                f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string()
            })
            .collect(),
        "f64" => values
            .chunks(4)
            .map(|c| {
                let bytes = to_bytes(c, byte_order);
                f64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ])
                .to_string()
            })
            .collect(),
        "string" => {
            let bytes: Vec<u8> = values
                .iter()
                .flat_map(|v| vec![(v >> 8) as u8, (v & 0xff) as u8])
                .collect();
            let trimmed: Vec<u8> = bytes.into_iter().filter(|&b| b != 0).collect();
            vec![String::from_utf8_lossy(&trimmed).to_string()]
        }
        _ => values.iter().map(|v| v.to_string()).collect(),
    }
}

fn to_bytes(regs: &[u16], order: &str) -> Vec<u8> {
    let ab: Vec<u8> = regs
        .iter()
        .flat_map(|v| vec![(v >> 8) as u8, (v & 0xff) as u8])
        .collect();
    let len = ab.len();
    match order {
        "ab" => ab,
        "ba" => {
            let mut out = vec![0u8; len];
            for (i, b) in ab.iter().enumerate() {
                out[len - 1 - i] = *b;
            }
            out
        }
        "abcd" => ab,
        "badc" => {
            let mut out = vec![0u8; len];
            for chunk in ab.chunks(2).enumerate() {
                let (i, pair) = chunk;
                if pair.len() == 2 {
                    out[i * 2] = pair[1];
                    out[i * 2 + 1] = pair[0];
                }
            }
            out
        }
        "cdab" => {
            let mut out = vec![0u8; len];
            for (i, b) in ab.iter().enumerate() {
                out[(i + 2) % len] = *b;
            }
            out
        }
        "dcba" => {
            let mut out = vec![0u8; len];
            for (i, b) in ab.iter().enumerate() {
                out[len - 1 - i] = *b;
            }
            out
        }
        _ => ab,
    }
}
