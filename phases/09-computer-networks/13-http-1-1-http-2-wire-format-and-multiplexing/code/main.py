"""HTTP/1.1 and HTTP/2 protocol utilities."""

from dataclasses import dataclass, field
from typing import Optional


# --- HTTP/1.1 ---

@dataclass
class HttpRequest:
    method: str
    path: str
    version: str
    headers: dict[str, str] = field(default_factory=dict)
    body: bytes = b""


@dataclass
class HttpResponse:
    status: int
    reason: str
    headers: dict[str, str] = field(default_factory=dict)
    body: bytes = b""


def parse_http_request(data: bytes) -> HttpRequest:
    """Parse raw bytes into an HTTP/1.1 request."""
    text = data.decode("utf-8", errors="replace")
    parts = text.split("\r\n\r\n", 1)
    header_section = parts[0]
    body = parts[1].encode() if len(parts) > 1 else b""

    lines = header_section.split("\r\n")
    request_line = lines[0].split(" ", 2)
    if len(request_line) < 3:
        raise ValueError(f"Invalid request line: {lines[0]}")

    method, path, version = request_line
    headers = {}
    for line in lines[1:]:
        if ":" in line:
            key, value = line.split(":", 1)
            headers[key.strip().lower()] = value.strip()

    return HttpRequest(method=method, path=path, version=version, headers=headers, body=body)


def build_http_response(response: HttpResponse) -> bytes:
    """Serialize an HTTP response to bytes."""
    lines = [f"HTTP/1.1 {response.status} {response.reason}"]
    for key, value in response.headers.items():
        lines.append(f"{key}: {value}")
    header_block = "\r\n".join(lines) + "\r\n\r\n"
    return header_block.encode() + response.body


def format_chunked(body: bytes, chunk_size: int = 16) -> bytes:
    """Encode a body using chunked transfer encoding."""
    result = []
    for i in range(0, len(body), chunk_size):
        chunk = body[i : i + chunk_size]
        result.append(f"{len(chunk):X}\r\n".encode() + chunk + b"\r\n")
    result.append(b"0\r\n\r\n")
    return b"".join(result)


# --- HTTP/2 ---

H2_FRAME_TYPES = {
    0x0: "DATA",
    0x1: "HEADERS",
    0x2: "PRIORITY",
    0x3: "RST_STREAM",
    0x4: "SETTINGS",
    0x5: "PUSH_PROMISE",
    0x6: "PING",
    0x7: "GOAWAY",
    0x8: "WINDOW_UPDATE",
    0x9: "CONTINUATION",
}

H2_PREFACE = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"


@dataclass
class H2Frame:
    length: int
    frame_type: int
    flags: int
    stream_id: int
    payload: bytes

    @property
    def type_name(self) -> str:
        return H2_FRAME_TYPES.get(self.frame_type, "UNKNOWN")

    @property
    def has_ack(self) -> bool:
        return bool(self.flags & 0x01)

    @property
    def is_end_stream(self) -> bool:
        return bool(self.flags & 0x01)

    @property
    def is_end_headers(self) -> bool:
        return bool(self.flags & 0x04)


def parse_h2_frame(data: bytes) -> Optional[H2Frame]:
    """Parse an HTTP/2 frame from raw bytes. Returns None if insufficient data."""
    if len(data) < 9:
        return None

    length = int.from_bytes(data[0:3], "big")
    frame_type = data[3]
    flags = data[4]
    stream_id = int.from_bytes(data[5:9], "big") & 0x7FFFFFFF

    if len(data) < 9 + length:
        return None

    payload = data[9 : 9 + length]
    return H2Frame(length=length, frame_type=frame_type, flags=flags, stream_id=stream_id, payload=payload)


def build_h2_frame(frame_type: int, flags: int, stream_id: int, payload: bytes) -> bytes:
    """Build an HTTP/2 frame as bytes."""
    length = len(payload)
    header = bytearray(9)
    header[0:3] = length.to_bytes(3, "big")
    header[3] = frame_type
    header[4] = flags
    header[5:9] = (stream_id & 0x7FFFFFFF).to_bytes(4, "big")
    return bytes(header) + payload


def build_settings_frame(settings: dict[int, int] | None = None) -> bytes:
    """Build a SETTINGS frame."""
    defaults = {3: 100, 4: 65535, 5: 16384}
    if settings:
        defaults.update(settings)

    payload = b""
    for setting_id, value in defaults.items():
        payload += setting_id.to_bytes(2, "big") + value.to_bytes(4, "big")

    return build_h2_frame(0x04, 0x00, 0, payload)


def parse_settings_payload(payload: bytes) -> dict[int, int]:
    """Parse SETTINGS frame payload into a dict."""
    settings = {}
    for i in range(0, len(payload), 6):
        if i + 6 <= len(payload):
            setting_id = int.from_bytes(payload[i : i + 2], "big")
            value = int.from_bytes(payload[i + 2 : i + 6], "big")
            settings[setting_id] = value
    return settings


# --- HPACK Simulation ---

STATIC_TABLE = [
    (":authority", ""),
    (":method", "GET"),
    (":method", "POST"),
    (":path", "/"),
    (":path", "/index.html"),
    (":scheme", "http"),
    (":scheme", "https"),
    (":status", "200"),
    (":status", "204"),
    (":status", "206"),
    (":status", "304"),
    (":status", "400"),
    (":status", "404"),
    (":status", "500"),
    ("accept-charset", ""),
    ("accept-encoding", "gzip, deflate"),
    ("accept-language", ""),
    ("accept-ranges", ""),
    ("accept", ""),
    ("access-control-allow-origin", ""),
    ("age", ""),
    ("allow", ""),
    ("authorization", ""),
    ("cache-control", ""),
    ("content-disposition", ""),
    ("content-encoding", ""),
    ("content-language", ""),
    ("content-length", ""),
    ("content-location", ""),
    ("content-range", ""),
    ("content-type", ""),
    ("cookie", ""),
    ("date", ""),
    ("etag", ""),
    ("expect", ""),
    ("expires", ""),
    ("from", ""),
    ("host", ""),
    ("if-match", ""),
    ("if-modified-since", ""),
    ("if-none-match", ""),
    ("if-range", ""),
    ("if-unmodified-since", ""),
    ("last-modified", ""),
    ("link", ""),
    ("location", ""),
    ("max-forwards", ""),
    ("proxy-authenticate", ""),
    ("proxy-authorization", ""),
    ("range", ""),
    ("referer", ""),
    ("refresh", ""),
    ("retry-after", ""),
    ("server", ""),
    ("set-cookie", ""),
    ("strict-transport-security", ""),
    ("transfer-encoding", ""),
    ("user-agent", ""),
    ("vary", ""),
    ("via", ""),
    ("www-authenticate", ""),
]


def hpack_lookup(name: str, value: str) -> Optional[int]:
    """Look up a header in the HPACK static table. Returns index or None."""
    for i, (n, v) in enumerate(STATIC_TABLE, start=1):
        if n.lower() == name.lower() and (v == "" or v.lower() == value.lower()):
            return i
    return None


def hpack_compress(headers: dict[str, str]) -> list[tuple[str, str, Optional[int]]]:
    """Compress headers using static table lookup."""
    result = []
    for name, value in headers.items():
        index = hpack_lookup(name, value)
        result.append((name, value, index))
    return result


# --- Main Demo ---

def main():
    print("=== HTTP/1.1 and HTTP/2 Protocol Utilities ===\n")

    # HTTP/1.1 demo
    raw = b"POST /submit HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/json\r\nContent-Length: 18\r\n\r\n{\"action\":\"create\"}"
    req = parse_http_request(raw)
    print(f"HTTP/1.1 Request: {req.method} {req.path} {req.version}")
    print(f"  Headers: {dict(req.headers)}")
    print(f"  Body: {req.body.decode()}")

    resp = HttpResponse(
        status=201,
        reason="Created",
        headers={"content-type": "application/json", "content-length": "15"},
        body=b'{"id": "abc123"}',
    )
    resp_bytes = build_http_response(resp)
    print(f"\nHTTP/1.1 Response:\n{resp_bytes.decode()}")

    # Chunked encoding demo
    chunked = format_chunked(b"Hello, chunked world!", chunk_size=8)
    print(f"Chunked encoding:\n{chunked.decode()}")

    # HTTP/2 frame demo
    settings = build_settings_frame()
    frame = parse_h2_frame(settings)
    if frame:
        print(f"HTTP/2 Frame: type={frame.type_name}, length={frame.length}, flags=0x{frame.flags:02X}, stream={frame.stream_id}")
        parsed_settings = parse_settings_payload(frame.payload)
        print(f"  Settings: {parsed_settings}")

    ping_payload = bytes.fromhex("DEADBEEF01020304")
    ping_frame = build_h2_frame(0x06, 0x00, 0, ping_payload)
    parsed = parse_h2_frame(ping_frame)
    if parsed:
        print(f"\nPING frame: payload={parsed.payload.hex()}")

    # HPACK demo
    print("\n--- HPACK Compression ---")
    headers = {":method": "GET", ":path": "/index.html", ":scheme": "https", "user-agent": "demo/1.0"}
    compressed = hpack_compress(headers)
    for name, value, index in compressed:
        if index:
            print(f"  {name}: {value} -> static table index {index}")
        else:
            print(f"  {name}: {value} -> literal (not in static table)")


if __name__ == "__main__":
    main()
