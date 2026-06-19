// WebSockets, SSE, gRPC — Real-time Protocol Utilities

// --- WebSocket Frame Parser ---

enum WsOpcode {
  Continuation = 0x0,
  Text = 0x1,
  Binary = 0x2,
  Close = 0x8,
  Ping = 0x9,
  Pong = 0xa,
}

function wsOpcodeName(opcode: WsOpcode): string {
  const names: Record<number, string> = {
    0x0: "Continuation",
    0x1: "Text",
    0x2: "Binary",
    0x8: "Close",
    0x9: "Ping",
    0xa: "Pong",
  };
  return names[opcode] ?? "Unknown";
}

interface WsFrame {
  fin: boolean;
  opcode: WsOpcode;
  masked: boolean;
  maskKey: Uint8Array | null;
  payload: Uint8Array;
}

function parseWsFrame(data: Uint8Array): { frame: WsFrame; consumed: number } {
  if (data.length < 2) throw new Error("Need at least 2 bytes");

  const fin = (data[0] & 0x80) !== 0;
  const opcode = data[0] & 0x0f;
  const masked = (data[1] & 0x80) !== 0;
  let payloadLen = data[1] & 0x7f;
  let offset = 2;

  if (payloadLen === 126) {
    if (data.length < 4) throw new Error("Need 4 bytes for 16-bit length");
    payloadLen = (data[2] << 8) | data[3];
    offset = 4;
  } else if (payloadLen === 127) {
    if (data.length < 10) throw new Error("Need 10 bytes for 64-bit length");
    payloadLen = 0;
    for (let i = 2; i < 10; i++) payloadLen = payloadLen * 256 + data[i];
    offset = 10;
  }

  let maskKey: Uint8Array | null = null;
  if (masked) {
    if (data.length < offset + 4) throw new Error("Need 4 bytes for mask key");
    maskKey = data.slice(offset, offset + 4);
    offset += 4;
  }

  const total = offset + payloadLen;
  if (data.length < total) throw new Error(`Need ${total} bytes, have ${data.length}`);

  const payload = data.slice(offset, total);
  if (maskKey) applyMask(payload, maskKey);

  return {
    frame: { fin, opcode: opcode as WsOpcode, masked, maskKey, payload },
    consumed: total,
  };
}

function buildWsFrame(opcode: WsOpcode, payload: Uint8Array, fin = true): Uint8Array {
  const len = payload.length;
  let headerLen = 2;
  if (len >= 65536) headerLen = 10;
  else if (len >= 126) headerLen = 4;

  const buf = new Uint8Array(headerLen + len);
  buf[0] = (fin ? 0x80 : 0x00) | opcode;

  if (len < 126) {
    buf[1] = len;
  } else if (len < 65536) {
    buf[1] = 126;
    buf[2] = (len >> 8) & 0xff;
    buf[3] = len & 0xff;
  } else {
    buf[1] = 127;
    for (let i = 7; i >= 0; i--) {
      buf[10 - i] = (len / Math.pow(256, i)) & 0xff;
    }
  }

  buf.set(payload, headerLen);
  return buf;
}

function applyMask(data: Uint8Array, key: Uint8Array): void {
  for (let i = 0; i < data.length; i++) {
    data[i] ^= key[i % 4];
  }
}

// --- SSE Event Parser ---

interface SseEvent {
  eventType: string;
  data: string;
  id: string | null;
  retry: number | null;
}

function parseSseEvent(lines: string[]): SseEvent {
  let eventType = "message";
  const dataParts: string[] = [];
  let id: string | null = null;
  let retry: number | null = null;

  for (const line of lines) {
    if (line.startsWith("event: ")) {
      eventType = line.slice(7);
    } else if (line.startsWith("data: ")) {
      dataParts.push(line.slice(6));
    } else if (line.startsWith("id: ")) {
      id = line.slice(4);
    } else if (line.startsWith("retry: ")) {
      retry = parseInt(line.slice(7), 10);
    }
  }

  return { eventType, data: dataParts.join("\n"), id, retry };
}

function parseSseStream(text: string): SseEvent[] {
  const events: SseEvent[] = [];
  let currentBlock: string[] = [];

  for (const line of text.split("\n")) {
    if (line === "") {
      if (currentBlock.length > 0) {
        events.push(parseSseEvent(currentBlock));
        currentBlock = [];
      }
    } else {
      currentBlock.push(line);
    }
  }

  if (currentBlock.length > 0) {
    events.push(parseSseEvent(currentBlock));
  }

  return events;
}

// --- gRPC Frame Parser ---

interface GrpcFrame {
  compressed: boolean;
  messageLength: number;
  message: Uint8Array;
}

function parseGrpcFrame(payload: Uint8Array): GrpcFrame {
  if (payload.length < 5) throw new Error("gRPC frame needs at least 5 bytes");

  const compressed = payload[0] !== 0;
  const messageLength =
    (payload[1] << 24) | (payload[2] << 16) | (payload[3] << 8) | payload[4];

  if (payload.length < 5 + messageLength) {
    throw new Error(`Need ${5 + messageLength} bytes, have ${payload.length}`);
  }

  const message = payload.slice(5, 5 + messageLength);
  return { compressed, messageLength, message };
}

function buildGrpcFrame(message: Uint8Array, compressed = false): Uint8Array {
  const buf = new Uint8Array(5 + message.length);
  buf[0] = compressed ? 1 : 0;
  buf[1] = (message.length >> 24) & 0xff;
  buf[2] = (message.length >> 16) & 0xff;
  buf[3] = (message.length >> 8) & 0xff;
  buf[4] = message.length & 0xff;
  buf.set(message, 5);
  return buf;
}

// --- Main Demo ---

function main(): void {
  console.log("=== WebSockets, SSE, gRPC (TypeScript) ===\n");

  // WebSocket demo
  console.log("--- WebSocket Frame Parser ---");
  const text = "Hello, WebSocket!";
  const encoder = new TextEncoder();
  const payload = encoder.encode(text);
  const frameBytes = buildWsFrame(WsOpcode.Text, payload);
  console.log(`Built Text frame: ${frameBytes.length} bytes`);

  const { frame, consumed } = parseWsFrame(frameBytes);
  console.log(`  FIN: ${frame.fin}`);
  console.log(`  Opcode: ${frame.opcode} (${wsOpcodeName(frame.opcode)})`);
  console.log(`  Masked: ${frame.masked}`);
  console.log(`  Length: ${frame.payload.length}`);
  console.log(`  Payload: ${new TextDecoder().decode(frame.payload)}`);
  console.log(`  Consumed: ${consumed} bytes`);

  // Masking demo
  const maskData = encoder.encode("mask me");
  const maskKey = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
  console.log("\n  Masking demo:");
  console.log(`    Original: [${Array.from(maskData).map(b => b.toString(16).padStart(2, "0")).join(", ")}]`);
  applyMask(maskData, maskKey);
  console.log(`    Masked:   [${Array.from(maskData).map(b => b.toString(16).padStart(2, "0")).join(", ")}]`);
  applyMask(maskData, maskKey);
  console.log(`    Unmasked: [${Array.from(maskData).map(b => b.toString(16).padStart(2, "0")).join(", ")}]`);

  // SSE demo
  console.log("\n--- SSE Event Parser ---");
  const sseText = `event: stock
data: {"symbol":"AAPL","price":185.5}
id: 42

event: stock
data: {"symbol":"GOOG","price":141.2}
id: 43

event: alert
data: Market closing soon
id: 44
retry: 5000
`;

  const events = parseSseStream(sseText);
  for (const ev of events) {
    console.log(`  Event: ${ev.eventType}`);
    console.log(`    Data: ${ev.data}`);
    if (ev.id) console.log(`    ID: ${ev.id}`);
    if (ev.retry !== null) console.log(`    Retry: ${ev.retry}ms`);
  }

  // gRPC demo
  console.log("\n--- gRPC Frame Parser ---");
  const grpcMsg = encoder.encode("grpc-service-call-data");
  const grpcFrame = buildGrpcFrame(grpcMsg);
  console.log(`Built gRPC frame: ${grpcFrame.length} bytes`);

  const parsed = parseGrpcFrame(grpcFrame);
  console.log(`  Compressed: ${parsed.compressed}`);
  console.log(`  Length: ${parsed.messageLength}`);
  console.log(`  Message: ${new TextDecoder().decode(parsed.message)}`);

  // Protocol comparison
  console.log("\n--- Protocol Comparison ---");
  console.log("  WebSocket:  Full-duplex, binary + text, ping/pong keep-alive");
  console.log("  SSE:        Server-to-client, text only, auto-reconnect");
  console.log("  gRPC:       RPC over HTTP/2, protobuf, 4 streaming modes");
}

main();
