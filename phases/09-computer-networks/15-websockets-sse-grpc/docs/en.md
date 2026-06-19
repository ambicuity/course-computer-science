# Lesson 15: WebSockets, SSE, gRPC

## Overview

HTTP is request-response: the client asks, the server answers. Modern applications need real-time, bidirectional, or streaming communication. Three protocols address this: **WebSockets** for full-duplex, **Server-Sent Events (SSE)** for server-to-client streaming, and **gRPC** for structured RPC over HTTP/2.

## WebSockets: Full-Duplex Communication

WebSocket creates a persistent, full-duplex channel over a single TCP connection. Once established, both client and server can send messages at any time with minimal overhead.

### WebSocket Handshake

The connection starts as an HTTP upgrade request:

```
GET /chat HTTP/1.1\r\n
Host: server.example.com\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n
Sec-WebSocket-Version: 13\r\n
\r\n
```

The server responds with:

```
HTTP/1.1 101 Switching Protocols\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n
\r\n
```

`Sec-WebSocket-Accept` is computed by concatenating the client key with a magic GUID, then taking the SHA-1 hash and base64 encoding it. This prevents caching proxies from misinterpreting WebSocket frames as HTTP.

### WebSocket Frame Format

After the handshake, data is transmitted in binary frames. The frame header is at least 2 bytes: byte 1 has FIN (1 bit), RSV (3 bits), opcode (4 bits); byte 2 has MASK (1 bit) and payload length (7 bits). Length extends to 16 or 64 bits for larger payloads. If MASK is set, a 4-byte masking key follows, and the payload is XOR'd with it.

Key fields:
- **FIN** (1 bit): final fragment of a message
- **Opcode** (4 bits): 0x1 text, 0x2 binary, 0x8 close, 0x9 ping, 0xA pong
- **MASK** (1 bit): client-to-server frames must be masked
- **Payload length**: 7 bits, or extended to 16/64 bits for larger payloads
- **Masking key**: 4 bytes, XOR'd with payload data

### Ping/Pong

Either side can send a **ping** frame; the other must respond with a **pong**. This serves as a keep-alive and connection health check.

### Close Handshake

Either side sends a close frame (opcode 0x8) with an optional status code and reason. The other side responds with a close frame, then both sides close the TCP connection.

## Server-Sent Events (SSE): Unidirectional Streaming

SSE is simpler than WebSockets. The server pushes events to the client over a standard HTTP connection. No upgrade is needed.

### Event Format

```
HTTP/1.1 200 OK\r\n
Content-Type: text/event-stream\r\n
Cache-Control: no-cache\r\n
Connection: keep-alive\r\n
\r\n
event: update\r\n
data: {"temperature": 72.5}\r\n
id: 42\r\n
\r\n
event: alert\r\n
data: High temperature warning\r\n
id: 43\r\n
\r\n
```

Event fields:
- **event**: event type (optional, defaults to "message")
- **data**: the payload
- **id**: for reconnection — client sends `Last-Event-ID` header on reconnect
- **retry**: reconnection interval in milliseconds

### Auto-Reconnect and SSE vs WebSockets

The browser's `EventSource` API automatically reconnects if the connection drops, sending the `Last-Event-ID` header so the server can resume from where it left off.

| Feature | SSE | WebSockets |
|---|---|---|
| Direction | Server → Client | Bidirectional |
| Protocol | HTTP/1.1 | WebSocket (after upgrade) |
| Reconnect | Automatic | Manual |
| Binary data | Text only | Text + Binary |
| Complexity | Simple | Moderate |
| Use case | Notifications, feeds | Chat, gaming, collaboration |

## gRPC: RPC over HTTP/2

gRPC is Google's Remote Procedure Call framework. It uses HTTP/2 for transport and Protocol Buffers (protobuf) for serialization, providing efficient, strongly-typed communication.

### Protocol Buffers

Define services and messages in `.proto` files:

```protobuf
syntax = "proto3";

service UserService {
  rpc GetUser (GetUserRequest) returns (User);
  rpc ListUsers (ListUsersRequest) returns (stream User);
  rpc UpdateUser (stream UserUpdate) returns (UpdateResult);
  rpc Chat (stream ChatMessage) returns (stream ChatMessage);
}

message User {
  string id = 1;
  string name = 2;
  string email = 3;
}

message GetUserRequest {
  string user_id = 1;
}
```

### Streaming Modes

gRPC supports four communication patterns:

1. **Unary**: single request → single response (like a function call)
2. **Server streaming**: single request → stream of responses
3. **Client streaming**: stream of requests → single response
4. **Bidirectional streaming**: stream of requests → stream of responses

### Why gRPC over HTTP/2

- **Multiplexing**: multiple RPC calls on one TCP connection
- **Header compression**: HPACK reduces overhead
- **Streaming**: HTTP/2 supports bidirectional streaming natively
- **Binary protocol**: protobuf is more compact than JSON
- **Code generation**: `.proto` files generate client/server code in any language

### gRPC vs REST

| Feature | gRPC | REST (JSON over HTTP) |
|---|---|---|
| Serialization | Protobuf (binary) | JSON (text) |
| Streaming | Native (4 modes) | Limited (SSE, WebSockets) |
| Contract | `.proto` file | OpenAPI/Swagger (optional) |
| Browser support | Needs gRPC-Web proxy | Native |
| Performance | Higher | Lower |

## Build It

See the code file for:
- WebSocket frame parser: decode opcode, payload length, masking key, and unmask data
- SSE event parser: parse `event:`, `data:`, `id:` fields from a stream
- gRPC concepts: protobuf message structure demonstration

## Use It

- **Chat applications**: WebSockets for real-time bidirectional messaging
- **Live dashboards**: SSE for stock tickers, sports scores, notification feeds
- **Microservices**: gRPC for internal service-to-service communication
- **Collaborative editing**: WebSockets for shared document state

## Ship It

Each protocol solves a different problem. Choosing the right one depends on directionality, browser support, and performance requirements.

## Exercises

### Level 1 — Recall

1. What HTTP header initiates a WebSocket upgrade?
2. What content type is used for SSE responses?
3. What serialization format does gRPC use?

### Level 2 — Application

4. Write a WebSocket frame decoder that extracts the opcode and unmask the payload.
5. Parse an SSE event stream and extract event types and data fields.
6. Explain why gRPC uses HTTP/2 instead of HTTP/1.1.

### Level 3 — Creation

7. Build a WebSocket masking function that applies XOR with a 4-byte key to a payload.
8. Design an SSE reconnection handler: define retry logic, Last-Event-ID tracking, and backoff strategy.
9. Create a protobuf-style message schema for a simple e-commerce order service with unary and streaming RPCs.
