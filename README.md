# Modbus Master Simulation (Tauri)

[English](./README.md) | [简体中文](./README.zh-CN.md)

A cross-platform [Modbus](https://en.wikipedia.org/wiki/Modbus) master simulator rebuilt with [Tauri 2](https://v2.tauri.app/) and React. This project is inspired by the original [GitHubDragonFly/ModbusMaster](https://github.com/GitHubDragonFly/ModbusMaster) Windows/Mono application, but rewritten as a modern, lightweight desktop app using Rust for the backend and a web-based UI.

## Features

- **Protocols**: TCP, UDP, and RTU (ASCII variants and RTU-over-TCP/UDP can be added in future releases).
- **Slave ID**: configurable Unit ID (1–247) and basic slave scanning.
- **Function codes**: read/write Coils, Discrete Inputs, Input Registers, and Holding Registers.
- **Data types**: UInt16, Int16, UInt32, Int32, Float32, Float64, and String with selectable byte order.
- **Connection management**: connect/disconnect, real-time status, and request logs.
- **Virtual serial ports**: serial port field can be typed manually to work with `com0com` or `tty0tty` pairs.
- **Multiple instances**: run multiple app instances side-by-side with different ports or serial ports.

## Supported Modbus Function Codes

- `01` Read Coils
- `02` Read Discrete Inputs
- `03` Read Holding Registers
- `04` Read Input Registers
- `05` Write Single Coil
- `06` Write Single Register
- `15` Write Multiple Coils
- `16` Write Multiple Registers

## Tech Stack

- **Backend**: Rust + Tauri 2 + Tokio
- **Modbus library**: [tokio-modbus](https://crates.io/crates/tokio-modbus)
- **Serial**: [tokio-serial](https://crates.io/crates/tokio-serial)
- **Frontend**: React 19 + TypeScript + Tailwind CSS + Vite

## Requirements

- [Node.js](https://nodejs.org/) (v20 or later recommended)
- [Rust](https://www.rust-lang.org/tools/install) toolchain (`cargo` + `rustc`)
- Windows, Linux, or macOS

## Development

```bash
# Ensure cargo is on your PATH (Windows PowerShell example)
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

npm install
npm run tauri dev
```

## Build

```bash
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
npm run tauri build
```

Build artifacts will be placed in `src-tauri/target/release/bundle/`.

## Download

Pre-built installers are available on the [Releases](https://github.com/hopekayo/modbus-master-simulation-tauri/releases) page.

## License

This project is licensed under the [MIT License](./LICENSE), matching the license of the original nModbus-based simulator.

## Acknowledgements

Original project: [GitHubDragonFly/ModbusMaster](https://github.com/GitHubDragonFly/ModbusMaster)

## Related Project

- [Modbus Slave Simulation (Tauri)](https://github.com/hopekayo/modbus-slave-simulation-tauri) — the counterpart Modbus slave simulator.
