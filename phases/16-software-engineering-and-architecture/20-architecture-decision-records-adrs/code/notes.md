# Notes — Architecture Decision Records (ADRs)

## ADR Template (Nygard Format)

Every ADR follows this structure:

```
# ADR-NNNN: [Short Noun Phrase]

## Status

[Proposed | Accepted | Deprecated | Superseded by ADR-XXXX]

## Context

[The forces at play — technical, business, organizational constraints.
Why is this decision necessary? What alternatives exist?]

## Decision

[The specific choice made and the reasoning behind it.]

## Consequences

[What happens now — positive, negative, and neutral outcomes.
Be honest about trade-offs.]
```

---

## Example ADR 001: Choosing PostgreSQL over MongoDB

```markdown
# ADR-0001: Use PostgreSQL for Persistent Storage

## Status

Accepted

## Context

We need a primary data store for the application. Key requirements:

- ACID compliance for financial transaction records
- Must run on AWS us-east-1
- Must support complex joins for reporting queries
- Team has 8 years of PostgreSQL experience, 0 MongoDB experience
- Project timeline allows 2 weeks for infrastructure setup
- Budget permits managed database service (RDS)

Alternatives considered:

- **MongoDB**: No team experience. Multi-document ACID transactions
  only available in version 4.0+ with replica sets, adding complexity.
  Reporting queries requiring joins would need application-level
  aggregation or denormalization.
- **MySQL**: Less feature-rich for our reporting needs (window functions,
  CTEs, JSONB). Team less familiar with MySQL-specific tooling.
- **DynamoDB**: No joins. Would require extensive denormalization and
  significantly more complex application logic for our reporting use case.

## Decision

We will use PostgreSQL 15 deployed as an AWS RDS instance with Multi-AZ
enabled. Connection pooling will be handled by PgBouncer.

## Consequences

- **Positive**: ACID compliance for financial data, team can be productive
  immediately, mature tooling (pg_dump, psql, pg_stat_statements),
  excellent JOIN performance for reporting queries, JSONB support for
  semi-structured data when needed.
- **Negative**: Write scaling is limited to vertical scaling unless we
  add Citus extension later, connection pooling requires PgBouncer as
  a sidecar, RDS storage costs scale with data volume, no built-in
  change data capture (requires Debezium).
- **Neutral**: We are committing to the PostgreSQL ecosystem for the
  foreseeable future. Migration to another database would require
  significant effort.
```

---

## Example ADR 002: Choosing REST over GraphQL for Public API

```markdown
# ADR-0002: Use REST for the Public API

## Status

Accepted

## Context

We are designing the public-facing API for our platform. The API will
be consumed by third-party developers building integrations. Key
requirements:

- The API must be easy to understand for developers unfamiliar with
  our domain.
- We need stable, versioned endpoints that we can maintain across
  releases.
- Our data model is relatively stable and does not change frequently.
- Most consumers need standard CRUD operations on well-defined resources.
- Team has REST API design experience; one engineer has GraphQL
  production experience.
- We need to ship the public API within 6 weeks.

Alternatives considered:

- **GraphQL**: Offers flexible queries and reduces over-fetching, but
  adds complexity for consumers unfamiliar with our schema. Caching
  is harder (no standard HTTP caching). Rate limiting is more complex
  (query-based vs. endpoint-based). Our team has limited production
  GraphQL experience.
- **gRPC**: Excellent for internal service-to-service communication
  (see ADR-0004), but protobuf is unfamiliar for most external
  developers and offers poor browser compatibility without a proxy.
- **REST + OData**: Too much flexibility for a public API; OData
  query syntax is complex and opens up difficult-to-optimize queries.

## Decision

We will design the public API as a RESTful API following the
JSON:API specification. Endpoints will be versioned via URL path
(`/v1/`, `/v2/`). We will use OpenAPI 3.1 for specification and
generate documentation with Redoc.

## Consequences

- **Positive**: Familiar paradigm for external developers, strong
  HTTP caching (ETags, Cache-Control), simple rate limiting per
  endpoint, well-established security patterns (OAuth2 scopes map
  naturally to endpoints), excellent tooling (Postman, curl, etc.).
- **Negative**: Clients may over-fetch or under-fetch data compared
  to GraphQL; we will mitigate this with sparse fieldsets and
  inclusion of related resources per JSON:API. Adding new fields
  requires version bumps for breaking changes. Multiple round-trips
  may be needed for complex data requirements.
- **Neutral**: We may revisit GraphQL for an internal developer API
  in the future (see ADR-0006). This decision applies specifically
  to the public-facing API.
```

---

## Example ADR 003: Choosing Monorepo over Polyrepo

```markdown
# ADR-0003: Use Monorepo for All Services

## Status

Accepted

## Context

Our organization has 12 microservices and 3 shared libraries. Changes
frequently span multiple services — a schema change in the User service
requires updates in the Auth and Notification services. Currently, we
use 15 separate Git repositories (polyrepo approach). Key pain points:

- Coordinated changes across repos require opening 3-5 PRs manually.
- Shared library updates require publishing packages to npm and then
  updating consumer repos, creating a 1-2 day lag.
- Cross-repo refactoring is extremely difficult.
- CI/CD pipelines are duplicated across repos.
- Team is 20 engineers; we can manage monorepo complexity at this size.

Alternatives considered:

- **Polyrepo (current)**: Works for independent services but creates
  friction for coordinated changes and shared libraries.
- **Polyrepo with changesets**: Using Lerna or Turborepo in polyrepo
  mode — partially monorepo tooling without monorepo structure. Still
  requires package publishing for internal dependencies.
- **Monorepo with Bazel**: Full monorepo with Bazel for hermetic builds.
  Bazel has a steep learning curve; our team has no Bazel experience.
  Overkill for 15 packages.

## Decision

We will consolidate all services and libraries into a single monorepo
managed with Turborepo. Shared libraries will be internal packages
referenced via workspace protocol, not published to npm.

## Consequences

- **Positive**: Atomic commits across services, simpler dependency
  management for shared libraries, single CI/CD pipeline with
  Turborepo's incremental builds, easier cross-repo refactoring,
  unified code review and access control.
- **Negative**: Repository size will grow over time (mitigated by
  Git's pack files and sparse checkout). CI pipeline complexity
  increases — must use Turborepo's caching to avoid rebuilding
  everything on every change. Teams must coordinate merge conflicts
  more carefully. Repository permissions are all-or-nothing unless
  we adopt CODEOWNERS for finer-grained control.
- **Neutral**: If the organization grows beyond 100 engineers, we may
  need to revisit this decision and evaluate Bazel or split repos.
```

---

## ADR Quick Reference

| Field | Must Contain | Common Mistake |
|-------|-------------|----------------|
| Title | Short noun phrase naming the decision | Writing a vague title like "Database Choice" |
| Status | Current lifecycle state (Proposed/Accepted/Deprecated/Superseded) | Leaving status as "Proposed" after acceptance |
| Context | Specific constraints, requirements, and alternatives | Writing vague context like "we need a database" |
| Decision | Specific, actionable choice | Writing "we will evaluate options" |
| Consequences | Honest positives, negatives, and neutrals | Listing only positives, ignoring trade-offs |

## ADR Numbering Rules

- Sequential: ADR-0001, ADR-0002, ADR-0003...
- Never reuse a number — even for superseded ADRs
- Never delete an ADR — supersede it with a new one
- Zero-padded to 4 digits (ADR-0001, not ADR-1)

## ADR Status Transitions

```
Proposed ──→ Accepted ──→ Deprecated
                 │
                 └──→ Superseded by ADR-XXXX
```

A proposed ADR can also be rejected — keep it in the log as a record of a path not taken.