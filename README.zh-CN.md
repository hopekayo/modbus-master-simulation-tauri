# Modbus Master Simulation (Tauri)

[English](./README.md) | 简体中文

使用 [Tauri 2](https://v2.tauri.app/) 和 React 重新实现的跨平台 [Modbus](https://en.wikipedia.org/wiki/Modbus) 主站模拟器。本项目参考原 [GitHubDragonFly/ModbusMaster](https://github.com/GitHubDragonFly/ModbusMaster) Windows/Mono 应用程序，后端使用 Rust，前端使用现代化的 Web UI 重新编写。

## 功能特性

- **协议支持**：TCP、UDP、RTU（ASCII 变体及 RTU-over-TCP/UDP 可在后续版本中补充）。
- **从站地址**：可配置的 Unit ID（1–247），支持基础从站扫描。
- **功能码**：读写线圈、离散输入、输入寄存器和保持寄存器。
- **数据类型**：UInt16、Int16、UInt32、Int32、Float32、Float64、String，支持选择字节序。
- **连接管理**：连接/断开、实时状态显示和请求日志。
- **虚拟串口**：串口字段可手动输入，便于配合 `com0com` 或 `tty0tty` 虚拟串口对使用。
- **多实例**：可同时运行多个应用实例，使用不同端口或串口。

## 支持的 Modbus 功能码

- `01` 读线圈
- `02` 读离散输入
- `03` 读保持寄存器
- `04` 读输入寄存器
- `05` 写单个线圈
- `06` 写单个寄存器
- `15` 写多个线圈
- `16` 写多个寄存器

## 技术栈

- **后端**：Rust + Tauri 2 + Tokio
- **Modbus 库**：[tokio-modbus](https://crates.io/crates/tokio-modbus)
- **串口**：[tokio-serial](https://crates.io/crates/tokio-serial)
- **前端**：React 19 + TypeScript + Tailwind CSS + Vite

## 环境要求

- [Node.js](https://nodejs.org/)（建议 v20 或更高版本）
- [Rust](https://www.rust-lang.org/tools/install) 工具链（`cargo` + `rustc`）
- Windows、Linux 或 macOS

## 开发运行

```bash
# 确保 cargo 在 PATH 中（Windows PowerShell 示例）
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

npm install
npm run tauri dev
```

## 构建

```bash
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
npm run tauri build
```

构建产物将位于 `src-tauri/target/release/bundle/`。

## 下载

预编译的安装包可在 [Releases](https://github.com/hopekayo/modbus-master-simulation-tauri/releases) 页面下载。

## 许可证

本项目采用 [MIT 许可证](./LICENSE)，与原 nModbus 模拟器保持一致。

## 致谢

原项目：[GitHubDragonFly/ModbusMaster](https://github.com/GitHubDragonFly/ModbusMaster)

## 相关项目

- [Modbus Slave Simulation (Tauri)](https://github.com/hopekayo/modbus-slave-simulation-tauri) — 对应配套的 Modbus 从站模拟器。
