# API Reference Card — REST vs GraphQL vs gRPC

## Decision Matrix: Which Paradigm When?

| Use This | When You Need | Avoid If |
|----------|--------------|----------|
| **REST** | CRUD resources, HTTP caching, public API, curl/browser simplicity, wide tooling | Complex nested queries, real-time streaming, tight bandwidth constraints |
| **GraphQL** | Variable data shapes per client, mobile/BFF layer, aggregating multiple services, rapid UI iteration | Simple key-value lookups, need for built-in HTTP caching, team unfamiliar with schema-first design |
| **gRPC** | Inter-service communication, low-latency binary protocol, streaming (uni/bi-directional), polyglot code generation | Browser clients (needs gRPC-Web proxy), human-readable debugging, simple CRUD that REST handles fine |

## Quick Comparison

| Dimension | REST | GraphQL | gRPC |
|---|---|---|---|
| **Protocol** | HTTP/1.1 or HTTP/2 | HTTP/1.1 or HTTP/2 | HTTP/2 only |
| **Format** | JSON (usually) | JSON | Protobuf binary |
| **Endpoint** | Many (per resource) | One (`/graphql`) | One per service |
| **Payload size** | Medium (over-fetch) | Small (client selects) | Tiny (binary) |
| **Latency** | Medium | Medium (fewer round-trips) | Low (binary + HTTP/2) |
| **Streaming** | SSE or WebSocket (hacks) | Subscriptions over WebSocket | Native (4 modes) |
| **Caching** | HTTP built-in (ETag, Cache-Control) | Must implement manually | None built-in |
| **Code gen** | OpenAPI generators | Apollo codegen, graphql-codegen | Built-in (`protoc`) |
| **Learning curve** | Low | Medium | Medium-High |
| **Browser** | Native | Needs client lib | Needs gRPC-Web proxy |
| **Evolution** | Version endpoints | Add fields, deprecate never remove | Add field numbers, reserve removed |

## Versioning Strategies

| Strategy | Example | Best For | Trade-off |
|---|---|---|---|
| **URL path** | `/v2/users` | Public APIs, maximum clarity | URL pollution, not purely RESTful |
| **Query param** | `/users?version=2` | Quick internal use | Caches may ignore, not idiomatic |
| **Header** | `X-API-Version: 2` | Clean URLs, internal APIs | Invisible in browser, harder to debug |
| **Content negotiation** | `Accept: application/vnd.api.v2+json` | Pure REST, same URL for all versions | Complex to implement, opaque |

**Recommendation:** Use URL path versioning (`/v1/`, `/v2/`) for public REST APIs. For GraphQL, use schema evolution (add fields, deprecate but never remove). For gRPC, use protobuf field numbering (add new numbers, reserve retired ones).

## Backward Compatibility Rules

| Action | REST (JSON) | GraphQL | Protobuf (gRPC) | Breaking? |
|---|---|---|---|---|
| Add optional field | Safe — old clients ignore | Safe — old queries skip | Safe — new field number | **No** |
| Remove field | **Breaking** — old clients expect it | **Breaking** — old queries reference it | **Breaking** — old binaries read wrong type | **Yes** |
| Rename field | **Breaking** — key changes | **Breaking** — query breaks | **Breaking** — field number changes | **Yes** |
| Change field type | **Breaking** — type mismatch | **Breaking** — schema conflict | **Breaking** — wire format changes | **Yes** |

### Deprecation Workflow

1. **Mark deprecated** in schema (`@deprecated` in OpenAPI, `@deprecated` directive in GraphQL, `deprecated = true` option in protobuf).
2. **Log warnings** server-side when the deprecated field is accessed.
3. **Communicate** to all client teams: "Field X retires on date Y."
4. **Remove** after date Y (reserve the field number in proto).
5. **Never reuse** field numbers or names.

## HTTP Status Codes Cheat Sheet (REST)

| Code | Meaning | When to Use |
|---|---|---|
| `200` | OK | Successful GET, PUT, PATCH |
| `201` | Created | Successful POST |
| `204` | No Content | Successful DELETE |
| `400` | Bad Request | Invalid JSON, missing required fields |
| `401` | Unauthorized | Missing or invalid auth |
| `403` | Forbidden | Auth present but insufficient permissions |
| `404` | Not Found | Resource doesn't exist |
| `409` | Conflict | Duplicate resource, version mismatch |
| `422` | Unprocessable Entity | Valid JSON, failed business rules |
| `429` | Too Many Requests | Rate limited |
| `500` | Internal Server Error | Unexpected server failure |

## gRPC Status Codes Cheat Sheet

| Code | Name | When |
|---|---|---|
| `0` | OK | Success |
| `1` | CANCELLED | Client cancelled |
| `2` | UNKNOWN | Server error with no details |
| `3` | INVALID_ARGUMENT | Client sent bad data |
| `5` | NOT_FOUND | Resource doesn't exist |
| `6` | ALREADY_EXISTS | Duplicate resource |
| `7` | PERMISSION_DENIED | Auth present but insufficient |
| `16` | UNAUTHENTICATED | Missing or invalid auth |
| `13` | INTERNAL | Unexpected server failure |

## Hybrid Architecture Pattern

```
Browser ──REST──►┌──────────┐──gRPC──► Service A
                  │ API GW   │──gRPC──► Service B
Mobile ──GraphQL─►│          │──gRPC──► Service C
                  └──────────┘
```

- **REST** for external: caching, simplicity, browser-native.
- **GraphQL** for mobile/complex UI: exact data shapes, aggregation.
- **gRPC** for internal: low latency, strong types, streaming.

## Protobuf Field Rules

- **Field numbers are forever.** Once assigned, never reuse.
- **Adding = safe.** New field number, old clients ignore it.
- **Removing = breaking.** Reserve the number: `reserved N; reserved "name";`
- **Default values are invisible.** A field set to its default (0, "", false) is not serialized — use `optional` to distinguish "unset" from "zero."

## Key Takeaway

> There is no "best" paradigm. There is only the right paradigm for the boundary you're designing. REST for external simplicity, GraphQL for client flexibility, gRPC for internal performance. Version early, deprecate before you remove, and let the schema be the single source of truth.