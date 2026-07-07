import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const BAUD_OPTIONS = ["1200", "2400", "4800", "9600", "19200", "38400", "57600", "115200"];
const DATA_BITS_OPTIONS = ["5", "6", "7", "8"];
const PARITY_OPTIONS = ["None", "Even", "Odd"];
const STOP_BITS_OPTIONS = ["1", "2"];

const MODE_OPTIONS = [
  { key: "tcp", label: "TCP" },
  { key: "udp", label: "UDP" },
  { key: "rtu", label: "RTU" },
];

const FUNCTION_CODES = [
  { key: 0x01, label: "01 Read Coils" },
  { key: 0x02, label: "02 Read Discrete Inputs" },
  { key: 0x03, label: "03 Read Holding Registers" },
  { key: 0x04, label: "04 Read Input Registers" },
  { key: 0x05, label: "05 Write Single Coil" },
  { key: 0x06, label: "06 Write Single Register" },
  { key: 0x0f, label: "15 Write Multiple Coils" },
  { key: 0x10, label: "16 Write Multiple Registers" },
];

const DATA_TYPES = [
  { key: "u16", label: "UInt16" },
  { key: "i16", label: "Int16" },
  { key: "u32", label: "UInt32" },
  { key: "i32", label: "Int32" },
  { key: "f32", label: "Float32" },
  { key: "f64", label: "Float64" },
  { key: "string", label: "String" },
];

const BYTE_ORDERS = [
  { key: "ab", label: "AB" },
  { key: "ba", label: "BA" },
  { key: "abcd", label: "ABCD" },
  { key: "badc", label: "BADC" },
  { key: "cdab", label: "CDAB" },
  { key: "dcba", label: "DCBA" },
];

const MAX_LOGS = 2048;

function isReadFunctionCode(fc: number) {
  return fc === 0x01 || fc === 0x02 || fc === 0x03 || fc === 0x04;
}

function isWriteFunctionCode(fc: number) {
  return fc === 0x05 || fc === 0x06 || fc === 0x0f || fc === 0x10;
}

function defaultCountForType(fc: number, dataType: string) {
  if (!isReadFunctionCode(fc)) return 1;
  if (dataType === "u16" || dataType === "i16") return 1;
  if (dataType === "u32" || dataType === "i32" || dataType === "f32") return 2;
  if (dataType === "f64") return 4;
  if (dataType === "string") return 8;
  return 1;
}

function parseWriteValues(input: string, fc: number): number[] {
  if (fc === 0x05 || fc === 0x0f) {
    return input
      .split(/[,;\s]+/)
      .filter((x) => x !== "")
      .map((x) => (x === "1" || x.toLowerCase() === "true" ? 1 : 0));
  }
  return input
    .split(/[,;\s]+/)
    .filter((x) => x !== "")
    .map((x) => parseInt(x, 10));
}

function App() {
  const [mode, setMode] = useState("tcp");
  const [unitId, setUnitId] = useState("1");
  const [host, setHost] = useState("127.0.0.1");
  const [netPort, setNetPort] = useState("502");
  const [serialPorts, setSerialPorts] = useState<string[]>([]);
  const [serialPort, setSerialPort] = useState("");
  const [manualCom, setManualCom] = useState("");
  const [baudRate, setBaudRate] = useState("9600");
  const [dataBits, setDataBits] = useState("8");
  const [parity, setParity] = useState("None");
  const [stopBits, setStopBits] = useState("1");

  const [functionCode, setFunctionCode] = useState(0x03);
  const [address, setAddress] = useState("0");
  const [count, setCount] = useState("1");
  const [dataType, setDataType] = useState("u16");
  const [byteOrder, setByteOrder] = useState("ab");
  const [writeValues, setWriteValues] = useState("");

  const [results, setResults] = useState<string[]>([]);
  const [logs, setLogs] = useState<string[]>([]);
  const [status, setStatus] = useState("Disconnected");
  const [isConnected, setIsConnected] = useState(false);
  const [scanStart, setScanStart] = useState("1");
  const [scanEnd, setScanEnd] = useState("10");
  const [scanning, setScanning] = useState(false);
  const logsRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    refreshSerialPorts();
    const unlistenLog = listen<{ message: string }>("modbus-log", (event) => {
      addLog(event.payload.message);
    });
    const unlistenStatus = listen<{ message: string }>("modbus-status", (event) => {
      setStatus(event.payload.message);
    });
    return () => {
      unlistenLog.then((u) => u());
      unlistenStatus.then((u) => u());
    };
  }, []);

  useEffect(() => {
    if (logsRef.current) {
      logsRef.current.scrollTop = logsRef.current.scrollHeight;
    }
  }, [logs]);

  useEffect(() => {
    const fc = functionCode;
    if (isReadFunctionCode(fc)) {
      setCount(defaultCountForType(fc, dataType).toString());
    }
  }, [functionCode, dataType]);

  const addLog = (message: string) => {
    setLogs((prev) => {
      const next = [...prev, message];
      if (next.length > MAX_LOGS) {
        next.shift();
      }
      return next;
    });
  };

  async function refreshSerialPorts() {
    try {
      const ports = await invoke<string[]>("get_serial_ports");
      setSerialPorts(ports.length > 0 ? ports : ["none found"]);
      if (ports.length > 0) {
        setSerialPort(ports[0]);
      }
    } catch (e) {
      setSerialPorts(["none found"]);
    }
  }

  async function handleConnect() {
    const parsedUnitId = parseInt(unitId, 10);
    if (Number.isNaN(parsedUnitId) || parsedUnitId < 1 || parsedUnitId > 247) {
      alert("Unit ID must be between 1 and 247");
      return;
    }
    const config: any = { mode, unit_id: parsedUnitId };
    if (mode === "tcp" || mode === "udp") {
      config.network = { host, port: parseInt(netPort, 10) };
    } else {
      const portName = manualCom.trim() || serialPort;
      config.serial = {
        port: portName,
        baud_rate: parseInt(baudRate, 10),
        data_bits: parseInt(dataBits, 10),
        parity,
        stop_bits: parseInt(stopBits, 10),
      };
    }
    try {
      const result = await invoke<{ connected: boolean; details: string }>(
        "connect_master",
        { config }
      );
      setIsConnected(result.connected);
      setStatus(result.details);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function handleDisconnect() {
    try {
      const result = await invoke<{ connected: boolean; details: string }>(
        "disconnect_master"
      );
      setIsConnected(result.connected);
      setStatus(result.details);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  }

  async function handleRead() {
    const parsedAddress = parseInt(address, 10);
    const parsedCount = parseInt(count, 10);
    if (Number.isNaN(parsedAddress) || Number.isNaN(parsedCount) || parsedCount < 1) {
      alert("Invalid address or count");
      return;
    }
    try {
      const result = await invoke<{
        ok: boolean;
        message: string;
        address: number;
        count: number;
        values: string[];
      }>("read_master", {
        request: {
          function_code: functionCode,
          address: parsedAddress,
          count: parsedCount,
          data_type: dataType,
          byte_order: byteOrder,
        },
      });
      if (result.ok) {
        setResults((prev) => [
          `Read FC${functionCode} @${result.address} x${result.count}: ${result.values.join(", ")}`,
          ...prev,
        ]);
      } else {
        setResults((prev) => [`FAILED: ${result.message}`, ...prev]);
      }
    } catch (e) {
      setResults((prev) => [`ERROR: ${e}`, ...prev]);
    }
  }

  async function handleWrite() {
    const parsedAddress = parseInt(address, 10);
    if (Number.isNaN(parsedAddress)) {
      alert("Invalid address");
      return;
    }
    const values = parseWriteValues(writeValues, functionCode);
    if (values.length === 0) {
      alert("Please enter at least one value");
      return;
    }
    try {
      const result = await invoke<{ ok: boolean; message: string; address: number; count: number }>(
        "write_master",
        {
          request: {
            function_code: functionCode,
            address: parsedAddress,
            values,
          },
        }
      );
      if (result.ok) {
        setResults((prev) => [
          `Write FC${functionCode} @${result.address} x${result.count}: OK`,
          ...prev,
        ]);
      } else {
        setResults((prev) => [`FAILED: ${result.message}`, ...prev]);
      }
    } catch (e) {
      setResults((prev) => [`ERROR: ${e}`, ...prev]);
    }
  }

  async function handleScan() {
    const start = parseInt(scanStart, 10);
    const end = parseInt(scanEnd, 10);
    if (Number.isNaN(start) || Number.isNaN(end) || start < 1 || end > 247 || start > end) {
      alert("Invalid scan range (1-247)");
      return;
    }
    setScanning(true);
    try {
      const result = await invoke<{ found: number[]; message: string }>("scan_slaves", {
        start,
        end,
      });
      setResults((prev) => [`Scan ${start}-${end}: ${result.message}`, ...prev]);
    } catch (e) {
      setResults((prev) => [`Scan ERROR: ${e}`, ...prev]);
    } finally {
      setScanning(false);
    }
  }

  const isSerial = mode === "rtu";
  const serialDisabled = manualCom.trim() === "" && serialPort === "";

  return (
    <div className="flex flex-col h-screen bg-slate-900 text-slate-100 overflow-hidden">
      <div className="flex-none p-3 border-b border-slate-700">
        <div className="flex flex-wrap gap-4 items-start">
          <div className="flex-1 min-w-[280px] border border-slate-600 rounded p-3">
            <h2 className="text-sm font-semibold mb-2 text-sky-400">
              {isSerial ? "Serial" : "TCP / UDP"}
            </h2>
            {!isSerial ? (
              <div className="grid grid-cols-[80px_1fr] gap-2 items-center">
                <label className="text-xs">Remote IP</label>
                <input
                  value={host}
                  onChange={(e) => setHost(e.target.value)}
                  disabled={isConnected}
                  className="text-sm"
                />
                <label className="text-xs">Remote Port</label>
                <input
                  value={netPort}
                  onChange={(e) => setNetPort(e.target.value)}
                  disabled={isConnected}
                  className="text-sm"
                />
              </div>
            ) : (
              <div className="grid grid-cols-[80px_1fr] gap-2 items-center">
                <label className="text-xs">Port</label>
                <select
                  value={serialPort}
                  onChange={(e) => setSerialPort(e.target.value)}
                  disabled={isConnected || serialPorts.length === 0}
                  className="text-sm"
                >
                  {serialPorts.map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </select>
                <label className="text-xs">Baud</label>
                <select
                  value={baudRate}
                  onChange={(e) => setBaudRate(e.target.value)}
                  disabled={isConnected}
                  className="text-sm"
                >
                  {BAUD_OPTIONS.map((b) => (
                    <option key={b} value={b}>
                      {b}
                    </option>
                  ))}
                </select>
                <label className="text-xs">Data Bits</label>
                <select
                  value={dataBits}
                  onChange={(e) => setDataBits(e.target.value)}
                  disabled={isConnected}
                  className="text-sm"
                >
                  {DATA_BITS_OPTIONS.map((b) => (
                    <option key={b} value={b}>
                      {b}
                    </option>
                  ))}
                </select>
                <label className="text-xs">Parity</label>
                <select
                  value={parity}
                  onChange={(e) => setParity(e.target.value)}
                  disabled={isConnected}
                  className="text-sm"
                >
                  {PARITY_OPTIONS.map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </select>
                <label className="text-xs">Stop Bits</label>
                <select
                  value={stopBits}
                  onChange={(e) => setStopBits(e.target.value)}
                  disabled={isConnected}
                  className="text-sm"
                >
                  {STOP_BITS_OPTIONS.map((s) => (
                    <option key={s} value={s}>
                      {s}
                    </option>
                  ))}
                </select>
                <label className="text-xs">Manual COM</label>
                <input
                  value={manualCom}
                  onChange={(e) => setManualCom(e.target.value)}
                  placeholder="e.g. COM3 or /dev/tnt0"
                  disabled={isConnected}
                  className="text-sm"
                />
              </div>
            )}
            <div className="flex gap-2 mt-3">
              <button
                onClick={handleConnect}
                disabled={isConnected || (isSerial && serialDisabled)}
                className="px-3 py-1 bg-sky-600 hover:bg-sky-500 rounded text-sm font-medium disabled:opacity-50"
              >
                Connect
              </button>
              <button
                onClick={handleDisconnect}
                disabled={!isConnected}
                className="px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-sm font-medium disabled:opacity-50"
              >
                Disconnect
              </button>
              {isSerial && (
                <button
                  onClick={refreshSerialPorts}
                  disabled={isConnected}
                  className="px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-sm font-medium disabled:opacity-50"
                >
                  Refresh
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 min-w-[280px] border border-slate-600 rounded p-3">
            <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
              <label className="text-sm">Comm Mode</label>
              <select
                value={mode}
                onChange={(e) => setMode(e.target.value)}
                disabled={isConnected}
                className="text-sm"
              >
                {MODE_OPTIONS.map((m) => (
                  <option key={m.key} value={m.key}>
                    {m.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
              <label className="text-sm">Unit ID</label>
              <input
                type="number"
                min={1}
                max={247}
                value={unitId}
                onChange={(e) => setUnitId(e.target.value)}
                disabled={isConnected}
                className="text-sm"
              />
            </div>
            <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
              <label className="text-sm">Function</label>
              <select
                value={functionCode}
                onChange={(e) => setFunctionCode(parseInt(e.target.value, 10))}
                className="text-sm"
              >
                {FUNCTION_CODES.map((f) => (
                  <option key={f.key} value={f.key}>
                    {f.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
              <label className="text-sm">Address</label>
              <input
                type="number"
                min={0}
                max={65534}
                value={address}
                onChange={(e) => setAddress(e.target.value)}
                className="text-sm"
              />
            </div>
            {isReadFunctionCode(functionCode) && (
              <>
                <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
                  <label className="text-sm">Count</label>
                  <input
                    type="number"
                    min={1}
                    max={125}
                    value={count}
                    onChange={(e) => setCount(e.target.value)}
                    className="text-sm"
                  />
                </div>
                <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
                  <label className="text-sm">Data Type</label>
                  <select
                    value={dataType}
                    onChange={(e) => setDataType(e.target.value)}
                    className="text-sm"
                  >
                    {DATA_TYPES.map((t) => (
                      <option key={t.key} value={t.key}>
                        {t.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
                  <label className="text-sm">Byte Order</label>
                  <select
                    value={byteOrder}
                    onChange={(e) => setByteOrder(e.target.value)}
                    className="text-sm"
                  >
                    {BYTE_ORDERS.map((b) => (
                      <option key={b.key} value={b.key}>
                        {b.label}
                      </option>
                    ))}
                  </select>
                </div>
              </>
            )}
            {isWriteFunctionCode(functionCode) && (
              <div className="grid grid-cols-[100px_1fr] gap-2 items-center mb-2">
                <label className="text-sm">Values</label>
                <input
                  value={writeValues}
                  onChange={(e) => setWriteValues(e.target.value)}
                  placeholder={functionCode === 0x05 || functionCode === 0x0f ? "1,0,1" : "1,2,3"}
                  className="text-sm"
                />
              </div>
            )}
            <div className="flex gap-2 mt-3">
              {isReadFunctionCode(functionCode) && (
                <button
                  onClick={handleRead}
                  disabled={!isConnected}
                  className="px-3 py-1 bg-sky-600 hover:bg-sky-500 rounded text-sm font-medium disabled:opacity-50"
                >
                  Read
                </button>
              )}
              {isWriteFunctionCode(functionCode) && (
                <button
                  onClick={handleWrite}
                  disabled={!isConnected}
                  className="px-3 py-1 bg-sky-600 hover:bg-sky-500 rounded text-sm font-medium disabled:opacity-50"
                >
                  Write
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 min-w-[200px] border border-slate-600 rounded p-3">
            <h3 className="text-sm font-semibold mb-2 text-sky-400">Slave Scan</h3>
            <div className="grid grid-cols-[60px_1fr_60px_1fr] gap-2 items-center mb-2">
              <label className="text-sm">Start</label>
              <input
                type="number"
                min={1}
                max={247}
                value={scanStart}
                onChange={(e) => setScanStart(e.target.value)}
                disabled={scanning}
                className="text-sm"
              />
              <label className="text-sm">End</label>
              <input
                type="number"
                min={1}
                max={247}
                value={scanEnd}
                onChange={(e) => setScanEnd(e.target.value)}
                disabled={scanning}
                className="text-sm"
              />
            </div>
            <button
              onClick={handleScan}
              disabled={!isConnected || scanning}
              className="px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-sm font-medium disabled:opacity-50"
            >
              {scanning ? "Scanning..." : "Scan"}
            </button>
          </div>
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-auto p-2 grid grid-cols-2 gap-2">
        <div className="border border-slate-600 rounded p-2 flex flex-col min-h-0">
          <h3 className="text-sm font-semibold mb-2 text-sky-400">Results</h3>
          <div className="flex-1 overflow-y-auto font-mono text-xs space-y-1">
            {results.length === 0 ? (
              <div className="text-slate-500">No operations yet.</div>
            ) : (
              results.map((r, i) => (
                <div key={i} className="border-b border-slate-700 pb-1">
                  {r}
                </div>
              ))
            )}
          </div>
        </div>

        <div className="border border-slate-600 rounded p-2 flex flex-col min-h-0">
          <h3 className="text-sm font-semibold mb-2 text-sky-400">Logs</h3>
          <div
            ref={logsRef}
            className="flex-1 overflow-y-auto font-mono text-xs space-y-1"
          >
            {logs.length === 0 ? (
              <div className="text-slate-500">No requests yet.</div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="truncate">
                  {log}
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      <div className="flex-none px-3 py-2 border-t border-slate-700">
        <div className="text-sm">
          Status: <span className="text-yellow-400">{status}</span>
        </div>
      </div>
    </div>
  );
}

export default App;
