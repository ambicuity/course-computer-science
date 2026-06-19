# API Design — REST, GraphQL, gRPC Trade-offs

> Three paradigms for building APIs, each born from a different pain. Picking the wrong one costs you months.

**Type:** Learn
**Languages:** TypeScript, Protobuf
**Prerequisites:** Phase 16 lessons 01–12
**Time:** ~75 minutes

## Learning Objectives

- Articulate the design principles common to all APIs: consistency, versioning, backward compatibility, documentation.
- Model the same user-service domain three ways — REST resources, GraphQL schema, gRPC protobuf — and explain when each is the right choice.
- Compare REST, GraphQL, and gRPC across payload size, latency, tooling, learning curve, and evolution.
- Choose an API versioning strategy (URL path, query param, header, content negotiation) and defend the choice.
- Apply backward-compatibility rules: adding fields is safe, removing fields is breaking, deprecation before removal.
- Design a hybrid architecture: REST for external clients, gRPC for internal services, GraphQL as a gateway aggregation layer.

## The Problem

You're building a platform with a public web app, a mobile client, and a dozen microservices. The web team wants REST because "browsers speak HTTP." The mobile team wants GraphQL because "we need exactly the fields we ask for." The platform team wants gRPC because "inter-service latency matters." Who is right?

All of them — but not for every endpoint. Choosing one paradigm for everything leads to either over-fetching (REST for complex queries), under-specifying (GraphQL for simple key-value lookups), or binary lock-in (gRPC for browser clients). This lesson gives you the decision framework.

## The Concept

### API Design Principles (Paradigm-Agnostic)

Regardless of whether you pick REST, GraphQL, or gRPC, four principles apply:

1. **Consistency** — Naming conventions, error shapes, pagination patterns, and authentication headers must be uniform across every endpoint. A client calling `GET /users` and `GET /orders` should encounter identical pagination semantics (`?cursor=abc`, same `total_count` field). Inconsistency forces clients to learn N APIs instead of 1.

2. **Versioning** — Every API changes. The question is how you signal which version a client expects. We'll cover four strategies later (URL path, query param, header, content negotiation). The key insight: versioning is not just a URL convention — it's a contract. The version identifier tells the server which set of invariants it must uphold.

3. **Backward Compatibility** — A change is backward-compatible if every client that worked before the change still works after it. Adding a new optional field is safe (old clients ignore it). Removing a field is breaking (old clients expect it). Renaming a field is breaking. Changing a field's type is breaking. Treat every breaking change as a major-version bump that requires client coordination.

4. **Documentation** — An undocumented API is an unreliable API. OpenAPI (for REST), GraphQL introspection, and protobuf descriptors each give machines a way to discover the API. But humans need narrative docs: what does this endpoint do? What are the edge cases? Auto-generated reference is a floor, not a ceiling.

### REST: Resources Over the Wire

REST (Representational State Transfer) models everything as resources identified by URLs, manipulated through a uniform set of HTTP methods.

**Resources** are nouns: `/users`, `/users/42`, `/users/42/orders`. A resource is anything you can name and CRUD.

**HTTP Methods** form the uniform interface:

| Method  | Semantics        | Idempotent | Safe |
|---------|-----------------|------------|------|
| GET     | Read            | Yes        | Yes  |
| POST    | Create / Action | No         | No   |
| PUT     | Replace         | Yes        | No   |
| PATCH   | Partial update  | No*        | No   |
| DELETE  | Remove          | Yes        | No   |

*PATCH is technically idempotent only if defined that way; in practice it's often not.

**Status Codes** carry meaning:

- `200 OK` — Success.
- `201 Created` — Resource created (POST).
- `204 No Content` — Success, no body (DELETE).
- `400 Bad Request` — Client sent invalid data.
- `401 Unauthorized` — Auth missing or invalid.
- `403 Forbidden` — Auth present but insufficient.
- `404 Not Found` — Resource doesn't exist.
- `409 Conflict` — State conflict (duplicate, version mismatch).
- `422 Unprocessable Entity` — Valid JSON, failed business rules.
- `429 Too Many Requests` — Rate limited.
- `500 Internal Server Error` — Unexpected server failure.

**HATEOAS** (Hypermedia as the Engine of Application State) is the constraint most REST APIs ignore. A HATEOAS-compliant response includes links to related actions:

```json
{
  "id": 42,
  "name": "Alice",
  "_links": {
    "self": { "href": "/users/42" },
    "orders": { "href": "/users/42/orders" },
    "deactivate": { "href": "/users/42/deactivate", "method": "POST" }
  }
}
```

Clients discover available transitions instead of hard-coding URLs. Most teams skip HATEOAS in practice, but understanding it clarifies what "RESTful" truly demands.

**When REST Works Best:**

- CRUD-heavy domains (resources map cleanly to HTTP verbs).
- HTTP caching matters (CDNs, proxies, `ETag`/`If-None-Match`).
- Simplicity and wide tooling support are priorities.
- Browser-native clients (fetch, curl, no special parsing).
- Public APIs where predictability beats flexibility.

### GraphQL: Ask for What You Need

GraphQL replaces many endpoints with one: a single query endpoint where the client specifies the shape of the response.

**Schema** defines the type system:

```graphql
type User {
  id: ID!
  name: String!
  email: String!
  orders: [Order!]!
}

type Order {
  id: ID!
  total: Float!
  items: [Item!]!
}

type Query {
  user(id: ID!): User
  users(filter: UserFilter): [User!]!
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  updateUser(id: ID!, input: UpdateUserInput!): User!
  deleteUser(id: ID!): Boolean!
}

type Subscription {
  userUpdated(id: ID!): User!
}
```

**Queries** are read operations. The client specifies exactly which fields it wants:

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    name
    email
  }
}
```

One request returns exactly `{ "name": "Alice", "email": "alice@example.com" }` — no over-fetching, no under-fetching.

**Mutations** are write operations. They follow a convention: read the result back in the same request.

```graphql
mutation CreateUser($input: CreateUserInput!) {
  createUser(input: $input) {
    id
    name
  }
}
```

**Subscriptions** are real-time streams over WebSocket:

```graphql
subscription OnUserUpdated($id: ID!) {
  userUpdated(id: $id) {
    name
    email
  }
}
```

**When GraphQL Works Best:**

- Complex, nested data requirements (mobile clients need specific fields).
- Aggregating data from multiple services in a single round-trip.
- Rapidly evolving front-ends that need new fields without new endpoints.
- Reducing over-fetching on bandwidth-constrained clients (mobile, IoT).
- Scenarios where schema-first design drives contract negotiation between teams.

### gRPC: Strongly-Typed, High-Performance RPC

gRPC uses Protocol Buffers (protobuf) as the interface definition language and binary format. It generates client and server stubs in 11+ languages.

**Protobuf** defines messages and services:

```protobuf
message User {
  int64 id = 1;
  string name = 2;
  string email = 3;
}

service UserService {
  rpc GetUser(GetUserRequest) returns (User);
  rpc CreateUser(CreateUserRequest) returns (User);
  rpc StreamUsers(StreamUsersRequest) returns (stream User);
}
```

Fields are numbered. Once assigned, a field number must never be reused (even after deletion). This is the core of protobuf backward compatibility.

**Streaming** comes in three flavors:
- **Unary** — Simple request/response (like REST).
- **Server streaming** — Client sends one request, server streams many responses.
- **Client streaming** — Client streams data, server responds once.
- **Bidirectional streaming** — Both sides stream independently.

**Code Generation** is the superpower. `protoc` emits typed clients/servers for Go, Java, Python, C++, TypeScript, etc. No hand-written serialization. No schema drift between client and server.

**When gRPC Works Best:**

- Inter-service communication (low latency, binary protocol, no browser needed).
- Strongly-typed contracts between services owned by different teams.
- Streaming use cases (live feeds, log tailing, file uploads).
- Polyglot environments where code generation eliminates boilerplate.
- Latency-sensitive paths where HTTP/JSON overhead is measurable.

### Comparison: REST vs GraphQL vs gRPC

| Dimension         | REST                    | GraphQL                 | gRPC                     |
|-------------------|-------------------------|-------------------------|--------------------------|
| **Payload size**  | Medium (JSON, over-fetch)| Small (client chooses)  | Tiny (binary protobuf)   |
| **Latency**       | Medium (HTTP/1.1 text)  | Medium (single round-trip) | Low (HTTP/2 binary)   |
| **Tooling**       | Excellent (curl, Postman, OpenAPI) | Good (GraphiQL, Apollo) | Niche (grpcurl, Evans) |
| **Learning curve**| Low                     | Medium                  | Medium-High              |
| **Evolution**     | Version endpoints       | Add fields to schema    | Add fields to proto      |
| **Browser support**| Native                | Needs client lib        | Needs gRPC-Web proxy     |
| **Caching**       | HTTP caching built-in   | Must implement manually  | No built-in cache        |
| **Code gen**      | OpenAPI generators      | Apollo codegen           | Built-in protoc          |
| **Streaming**     | SSE / WebSocket hacks   | Subscriptions (WebSocket)| First-class (4 modes)   |
| **Introspection** | OpenAPI spec            | Built-in __schema       |.proto files + reflection |

### The Hybrid Approach

No rule says you must pick one. Most production systems combine them:

```
┌───────────┐
│  Browser  │──REST──────►┐
└───────────┘              │
                           ▼
┌───────────┐        ┌──────────┐       ┌──────────┐
│  Mobile   │──GraphQL──►│ API GW   │────gRPC──►│ Service A│
└───────────┘        └──────────┘       └──────────┘
                           │
                           │gRPC
                           ▼
                      ┌──────────┐
                      │ Service B│
                      └──────────┘
```

- **REST** for external, browser-facing APIs where caching and simplicity dominate.
- **GraphQL** as an aggregation layer for mobile / complex UI clients.
- **gRPC** for internal inter-service communication where latency and type safety matter.

Each boundary is a contract. The API gateway translates between paradigms.

### API Versioning Strategies

1. **URL Path** — `/v1/users`, `/v2/users`
   - Pros: Explicit, cacheable, easy to understand.
   - Cons: URL pollution, not truly RESTful (same resource, different URLs).

2. **Query Parameter** — `/users?version=2`
   - Pros: Keeps URL clean.
   - Cons: Easily overlooked by caches and proxies, not idiomatic.

3. **Header** — `X-API-Version: 2` or `Accept: application/vnd.api.v2+json`
   - Pros: Clean URLs, supports content negotiation.
   - Cons: Invisible in browser address bar, harder to debug with curl.

4. **Content Negotiation** — `Accept: application/vnd.api.v2+json`
   - Pros: Most "RESTful" approach, same URL for all versions.
   - Cons: Complex to implement, opaque to debugging.

**Recommendation:** Use URL path versioning (`/v1/`, `/v2/`) for public REST APIs. It's the most discoverable, most debuggable, and most widely understood. For gRPC, versioning is handled by protobuf field numbers (add fields, never remove). For GraphQL, use schema evolution (add fields/types, deprecate but never remove).

### Backward Compatibility in Practice

**Adding a field is safe:**

```protobuf
// v1
message User {
  int64 id = 1;
  string name = 2;
}

// v2 — safe, old clients ignore the new field
message User {
  int64 id = 1;
  string name = 2;
  string email = 3;  // NEW
}
```

**Removing a field is breaking:**

```protobuf
// v1
message User {
  int64 id = 1;
  string name = 2;
  string email = 3;
}

// v2 — BREAKING: old clients still expect email
// WRONG: delete the field
// RIGHT: deprecate first, then remove after all clients migrate
message User {
  int64 id = 1;
  string name = 2;
  reserved 3;           // field number 3 is retired, never reused
  reserved "email";     // field name is retired
}
```

**Deprecation workflow:**
1. Mark the field `deprecated` in the schema.
2. Log warnings server-side when the field is accessed.
3. Communicate to all client teams: "Field X will be removed on date Y."
4. After date Y, replace the field with `reserved` in protobuf, or remove it from GraphQL schema.
5. Never reuse field numbers or field names.

### Documentation

**OpenAPI** (REST): Describe endpoints, request/response schemas, auth, and status codes in a YAML/JSON spec. Tools like Swagger UI render interactive docs. Keep the spec in version control alongside the code.

**GraphQL Introspection**: The `__schema` query returns the full type system at runtime. Tools like GraphiQL render interactive docs from introspection alone. Add descriptions via `"""doc strings"""` in the schema.

**Protobuf Descriptors**: The `.proto` file is the documentation. Generate HTML docs with `protoc --doc_out`. Add comments directly in the proto file — they become part of the generated descriptor.

Rule: If the doc is not generated from the schema, it will drift. Single source of truth = the schema.

### API Gateway Pattern

An API gateway sits between clients and services, handling:

- **Routing** — Map `/v1/users` to the user service, `/v1/orders` to the order service.
- **Protocol translation** — Accept REST from browsers, forward gRPC to services.
- **Auth** — Verify JWT tokens once at the gateway, pass identity downstream.
- **Rate limiting** — Per-client throttling before traffic hits services.
- **Logging / metrics** — Centralized request tracing.

Gateways do not replace versioning or backward compatibility. They enforce them.

## Build It

We'll model a **User Service** three ways in `code/main.ts` and define the protobuf contract in `code/main.proto`.

### Step 1: Minimal Version (REST)

A single Express-style handler for CRUD on users, using in-memory storage.

### Step 2: Realistic Version (GraphQL + gRPC)

Add a GraphQL schema with resolvers and gRPC service stubs that model the same domain. Show how the same data flows through three paradigms.

## Use It

- **REST in production**: GitHub's REST API (`developer.github.com`) is the gold standard for consistent REST. Look at their pagination (`Link` headers), error format, and `Accept` header versioning.
- **GraphQL in production**: Shopify's Storefront API demonstrates schema evolution — they add fields and deprecate rather than version.
- **gRPC in production**: Kubernetes uses gRPC for etcd communication, andIstio's Envoy proxy uses gRPC for xDS streaming.

Compare the lesson's proto file against the Kubernetes API proto definitions (`k8s.io/api/`) to see how production protobuf evolves over years of backward-compatible changes.

## Read the Source

- **`k8s.io/apimachinery/pkg/apis/meta/v1/generated.proto`** — A real-world protobuf definition that demonstrates field evolution over many Kubernetes releases. Notice how fields are deprecated but never deleted, and field numbers are never reused.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **API reference card (`api_reference.md`)** comparing REST, GraphQL, and gRPC with a decision matrix.

## Exercises

1. **Easy** — Implement a REST endpoint for `DELETE /users/:id` that returns the appropriate status codes for success (204), not-found (404), and auth errors (401/403).
2. **Medium** — Design a GraphQL schema for an e-commerce domain (Product, Order, Customer, Review) with at least one connection (e.g., Product → [Review]) and one mutation with input validation.
3. **Hard** — Design a protobuf service that supports bidirectional streaming for a real-time chat system. Handle the case where field numbers must be reserved after removal, and write the deprecation workflow for removing a `nickname` field from the `User` message.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| REST | "An API that returns JSON" | An architectural style where resources are identified by URLs, manipulated through a uniform set of methods, with hypermedia-driven state transitions |
| GraphQL | "A replacement for REST" | A query language and runtime that lets clients specify the exact shape of the response from a single endpoint |
| gRPC | "Google's RPC thing" | A high-performance RPC framework using HTTP/2, protobuf serialization, and code generation for strongly-typed contracts |
| HATEOAS | "Links in JSON" | Hypermedia as the Engine of Application State — clients discover actions from response links, not hard-coded URLs |
| Protobuf | "Google's JSON alternative" | A binary serialization format with a schema definition language, field-numbered evolution, and code generation in 11+ languages |
| Backward compatible | "The API still works" | A change that doesn't break any existing client — adding optional fields is safe; removing or renaming fields is breaking |
| API Gateway | "A reverse proxy" | A single entry point that handles routing, auth, rate limiting, and protocol translation across multiple backend services |
| Content negotiation | "Accept headers" | An HTTP mechanism where client and server agree on the representation format (JSON, protobuf, v1 vs v2) via headers |

## Further Reading

- **REST**: Roy Fielding's dissertation, Chapter 5 — the original definition of REST constraints.
- **GraphQL**: Spec at `spec.graphql.org` — the authoritative reference for types, queries, mutations, and subscriptions.
- **gRPC**: `grpc.io/docs` — the official docs covering all four streaming modes and protobuf wire format.
- **API Versioning**: `tkbb.github.io/api-versioning-methods` — a survey of versioning strategies with trade-offs.
- **Proto3 Language Guide**: `protobuf.dev/programming-guides/proto3` — field numbering rules that make backward compatibility possible.