# Versioning, Deprecation, Compatibility

> Why APIs break, how to prevent it, and what to do when you can't.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 lessons 01–13
**Time:** ~60 minutes

## Learning Objectives

- Explain why API versioning exists and the real cost of breaking changes.
- Apply semantic versioning correctly: what makes a change major, minor, or patch.
- Distinguish backward-compatible changes from breaking changes with concrete examples.
- Compare API versioning strategies (URL path, query param, header, content negotiation) and their tradeoffs.
- Design a deprecation workflow: announce, warn, sunset, remove.
- Describe database schema evolution patterns including expand/contract migrations.
- Use feature flags to decouple deployment from release.
- Design contract tests (Pact, consumer-driven) that catch breaking changes before they ship.

## The Problem

You built an API. 50 clients depend on it. Now you need to rename a field. Seems innocent—just change `name` to `full_name` and redeploy. Except now 50 clients are broken in production. Their code expects `name`, you gave them `full_name`, and you didn't even know which clients existed.

This is the versioning problem. It is not theoretical. Every team that owns an API used by others—internal microservices, public SDKs, mobile apps—hits it. The question isn't whether you'll need versioning, it's how much pain you'll feel when you do.

Without a deliberate versioning strategy, you get one of two outcomes:

1. **Fear of change.** Teams stop evolving the API because they can't predict what breaks. The API stagnates, accumulates warts, and becomes a liability.
2. **Silent breakage.** Teams deploy changes freely and break consumers they didn't know about. Incidents follow.

Neither is acceptable. This lesson teaches the discipline that avoids both.

## The Concept

### Why APIs Need Versioning

An API is a **contract** between a provider and a consumer. The contract says: "if you send me X, I'll respond with Y." Versioning is how you change that contract without betraying the trust of existing consumers.

The cost of a breaking change compounds:

- **N clients** each have **M hours** of work to adapt.
- You don't know N (shadow consumers exist).
- The blast radius is invisible until production breaks.
- Rollback is your only mitigation, and rollback means undoing the feature you just shipped.

### Semantic Versioning (SemVer)

SemVer gives a universal vocabulary for communicating the impact of a change.

**Format:** `MAJOR.MINOR.PATCH` (e.g., `2.4.1`)

| Component | Bumped when | Meaning |
|-----------|-------------|---------|
| **MAJOR** | Incompatible API changes | Consumers must change their code |
| **MINOR** | Backward-compatible new functionality | Consumers can upgrade safely; new opt-in features |
| **PATCH** | Backward-compatible bug fixes | No API surface changes; consumers can upgrade freely |

**The rule:** Given a version number A.B.C, a consumer pinned to A.x.y can safely upgrade to A.B'.C' where B' >= B. A major version bump means "I broke something you depend on."

SemVer works for libraries and packages where the consumer explicitly chooses a version. For HTTP APIs, the versioning strategies below are how you surface that same information.

### Backward Compatibility: What Makes a Change Safe vs. Breaking

A change is **backward-compatible** when every existing consumer continues working without modification. A change is **breaking** when at least one consumer must change their code to keep working.

#### Compatible Changes (safe)

- Adding a new optional field to a response (`"new_field": "value"`)
- Adding a new endpoint (`GET /v1/users/search`)
- Adding a new optional query parameter (`?expand=true`)
- Widening a type (field was `int`, now accepts `int | float`)
- Adding a new enum value to a field consumers ignore unknown values for
- Increasing a maximum page size limit

#### Breaking Changes (require coordination)

- Removing a field from a response
- Renaming a field (`name` → `full_name`)
- Changing a type (`string` → `int`, `int` → `string`)
- Changing semantics (field `status` values: `"active"` now means `"pending_verification"`)
- Making an optional field required
- Adding a new required field to a request
- Tightening a constraint (max page size 100 → max page size 10)
- Changing error codes or error response shapes
- Reordering fields in a protobuf or fixed-schema encoding
- Removing an endpoint entirely

**Rule of thumb:** If any consumer could have written code that depends on the current behavior and that code would break after your change, it's a breaking change—even if you think "nobody uses that."

### API Versioning Strategies

There are four mainstream strategies for surfacing API version information:

#### 1. URL Path Versioning

```
GET https://api.example.com/v1/users
GET https://api.example.com/v2/users
```

| Pros | Cons |
|------|------|
| Obvious, visible in logs and URLs | Implies different resources; violates REST principle that URL = resource identity |
| Easy to route (load balancer, proxy) | Forces URL changes; hard to version per-field |
| Cache-friendly (URL is cache key) | Multiple base URLs = multiple code paths to maintain |
| Easy to test: curl the URL | |

**Used by:** Twilio, SendGrid, many startups.

#### 2. Query Parameter Versioning

```
GET https://api.example.com/users?v=1
GET https://api.example.com/users?v=2
```

| Pros | Cons |
|------|------|
| URL stays the same (resource identity preserved) | Version is easy to miss; defaults can be surprising |
| Easy to add incrementally | Caching gets tricky (same URL, different responses) |
| | Not visible without inspecting the query string |

**Used by:** Google Data API, Amadeus.

#### 3. Header-Based Versioning

```
GET https://api.example.com/users
Accept: application/vnd.example.v2+json
```

Or with a custom header:

```
GET https://api.example.com/users
X-API-Version: 2
```

| Pros | Cons |
|------|------|
| Clean URLs (resource identity preserved) | Invisible in logs without custom logging; harder to debug with curl |
| Content negotiation is HTTP-standard | Clients must remember to send the header |
| Flexible: can version per content-type | Requires client library support; proxy/cache config is harder |

**Used by:** GitHub (via `Accept` header), Stripe (via `Stripe-Version`).

#### 4. Content Negotiation

```
GET https://api.example.com/users
Accept: application/vnd.example+json; version=2
```

This is a formalization of header-based versioning using the `Accept` header per RFC 7231. The server inspects the `Accept` header and returns the requested representation.

| Pros | Cons |
|------|------|
| Most "correct" per HTTP spec | Most complex to implement correctly |
| Same URL = same resource, different representation | Harder to test manually |
| Supports multiple formats natively | Many developers are unfamiliar with content negotiation |

**Used by:** GitHub (their `Accept` header approach).

#### Choosing a Strategy

For **public APIs**, prioritize visibility and ease of use: URL path or header. For **internal APIs**, header-based works well because you control both sides. Avoid query parameters unless you need quick-and-dirty versioning without changing routing.

The best strategy is the one your consumers can actually use correctly. A theoretically pure strategy that confuses callers is worse than a pragmatic one that everyone understands.

### Deprecation Workflow

Deprecation is the art of removing something without angering the people who depend on it.

#### The Four-Phase Workflow

1. **Announce** — Document that a feature will be removed. Update API docs, changelogs, and release notes. Give a timeline.
2. **Warn** — At runtime, signal that the feature is deprecated. Add response headers. Log warnings server-side. Emit metrics.
3. **Sunset** — The feature still works but is on a timer. After the sunset date, it stops working. The sunset date was communicated in phase 1.
4. **Remove** — Delete the code. The feature no longer exists.

#### Deprecation Headers

**`Deprecation` header** (RFC 8594):

```
HTTP/1.1 200 OK
Deprecation: true
Link: <https://api.example.com/docs/deprecations/users-endpoint>; rel="deprecation"
Sunset: Sat, 01 Nov 2025 00:00:00 GMT
```

- `Deprecation: true` — tells the consumer this endpoint/feature is on the way out.
- `Link: ...; rel="deprecation"` — points to migration documentation.
- `Sunset` — the date-time after which the endpoint will no longer be available.

These headers let automated tools detect deprecation and alert teams before it's too late.

#### How Long Should the Deprecation Period Be?

It depends on your consumers:

| Consumer type | Recommended minimum | Rationale |
|--------------|---------------------|-----------|
| Internal services | 2–4 weeks | You control deployments; can coordinate directly |
| Mobile apps | 6–12 months | Users may not update apps for months; old versions persist |
| Third-party integrations | 6–12 months | External teams need planning cycles; you don't control their roadmap |
| Open-source libraries (SemVer) | Until next major version | SemVer promises no breaking changes within major version |

**The rule:** The deprecation period must be at least as long as your slowest reasonable consumer's update cycle. If you can't measure that, default to 6 months.

### Database Schema Evolution

APIs sit on top of databases, and database schemas change for the same reasons APIs do. But databases have an extra constraint: existing data must not be lost or corrupted.

#### Safe Schema Changes

- **Adding a nullable column** — No existing rows are affected; new rows treat it as `NULL`.
- **Adding a new table** — Nothing depends on it yet.
- **Adding an index** — Read performance may change during creation, but correctness is unaffected.
- **Widening a column type** (`VARCHAR(50)` → `VARCHAR(200)`) — Existing data fits in the wider type.

#### Breaking Schema Changes

- **Renaming a column** — All queries referencing the old name break.
- **Removing a column** — Data is lost; queries referencing it break.
- **Changing a column type** (`VARCHAR` → `INT`) — Existing data may not convert.
- **Making a nullable column NOT NULL** — Rows with `NULL` values violate the constraint.

#### The Expand/Contract Pattern

The expand/contract pattern (also called parallel change) safely applies breaking schema changes across three phases:

**Phase 1 — Expand:** Add the new schema alongside the old. Both exist simultaneously.

```
-- Add new column (nullable, alongside old one)
ALTER TABLE users ADD COLUMN full_name VARCHAR(200);
```

**Phase 2 — Migrate:** Move data from the old schema to the new. Update application code to write to both columns. Deploy code that reads from the new column.

```
-- Backfill new column from old
UPDATE users SET full_name = name WHERE full_name IS NULL;
-- Application code now writes to BOTH columns
```

**Phase 3 — Contract:** Remove the old schema. The new schema stands alone.

```
-- Application no longer references 'name' column
ALTER TABLE users DROP COLUMN name;
```

This pattern means the database is never in an intermediate broken state. The application is always compatible with both the old and new schema during the transition.

### Feature Flags for Gradual Rollout

Feature flags decouple **deployment** (getting code to production) from **release** (making a feature visible to users). This is essential for versioning because:

1. You can deploy a new API version behind a flag and enable it for a single consumer for testing.
2. You can roll back by disabling the flag—no code rollback needed.
3. You can measure the impact of a version change on a small percentage of traffic before going full.

**Pattern: Canary a new API version**

```
Request → Feature flag check
            ├── flag=v2 for customer_id in [A, B, C] → Route to v2 handler
            └── default → Route to v1 handler
```

**When to remove the flag:** Once all consumers have migrated to v2 and v1 traffic is zero, remove the flag and the v1 code path. A flag that lives forever is technical debt.

### Contract Testing

#### The Problem Contract Testing Solves

Integration tests tell you that the provider's current code works with the consumer's current code. But they don't tell you whether the provider's *next* deployment will break the consumer. Contract tests do.

#### Consumer-Driven Contracts (Pact)

In consumer-driven contract testing:

1. **The consumer writes a contract** that describes what it expects from the provider (e.g., "when I call `GET /users/1`, I get back an object with a `name` field that is a string").
2. **The contract is published** to a shared broker.
3. **The provider verifies** that it satisfies the contract on every build.

This catches breaking changes **before they deploy**, not after.

**Example Pact (simplified):**

```json
{
  "consumer": "order-service",
  "provider": "user-api",
  "interactions": [
    {
      "description": "a request for user 1",
      "request": { "method": "GET", "path": "/users/1" },
      "response": {
        "status": 200,
        "headers": { "Content-Type": "application/json" },
        "body": { "id": 1, "name": "Ada Lovelace" }
      }
    }
  ]
}
```

If the provider removes the `name` field or changes its type, the contract verification fails in CI, and the provider team knows before their change ships.

#### Key Practices

- **Each consumer owns its contract.** The provider doesn't guess what consumers need; consumers state their needs.
- **Contracts live in CI, not just as documentation.** A contract that isn't verified automatically is just a wish.
- **Verification runs on the provider's build pipeline.** The provider proves, on every build, that it hasn't broken any consumer contract.

### Real Examples

#### Stripe — Date-Based Versioning

Stripe versions its API by date: `Stripe-Version: 2023-08-16`. When you make a request without a version header, you get the oldest version of the API. This means:

- Stripe never breaks existing integrations. Your integration is pinned to the date you onboarded.
- Stripe accumulates versioned behavior internally and can sunset old versions after a deprecation period.
- You can upgrade at your own pace by changing the version header.

This is effectively header-based versioning where the "version" is a datestamp. The elegance: you always know exactly which version you're on, and Stripe can evolve the API continuously without coordinating with consumers.

#### GitHub — Header-Based Versioning

GitHub uses content negotiation via the `Accept` header:

```
Accept: application/vnd.github.v3+json
```

This is "media type" versioning. Benefits:

- The URL never changes (`https://api.github.com/repos/owner/repo`).
- The version is explicit in every request.
- GitHub can add new fields to responses without breaking old consumers (additive changes are compatible).
- Breaking changes result in a new media type version.

#### Kubernetes — API Groups

Kubernetes uses API groups to organize and version its massive API surface:

```
/api/v1                          (core group)
/apis/apps/v1                    (apps group)
/apis/batch/v1                   (batch group)
/apis/networking.k8s.io/v1       (networking group)
```

Each API group is versioned independently. This means:

- `apps/v1` can evolve without affecting `batch/v1`.
- Alpha (`v1alpha1`), beta (`v1beta1`), and stable (`v1`) versions coexist.
- Kubernetes can promote an API from alpha → beta → stable incrementally.
- Old versions are deprecated and eventually removed across several Kubernetes releases.

This is URL path versioning applied at the group level. The hierarchy makes it navigable despite hundreds of endpoints.

#### Twilio — URL-Based Versioning

Twilio uses URL path versioning:

```
https://api.twilio.com/2010-04-01/Accounts/{Sid}/Messages
```

The version is `2010-04-01`, a date-based path segment. Like Stripe's approach, this means:

- Existing integrations keep working indefinitely.
- Twilio can introduce new versions alongside old ones.
- The version is visible in every URL, making debugging straightforward.

The cost: Twilio maintains multiple API versions in production indefinitely. The `2010-04-01` version has been running for over a decade.

### The Cost of Maintaining Multiple Versions

Maintaining multiple API versions is expensive. The costs include:

1. **Code complexity.** Every endpoint handler must support multiple request/response shapes, or you maintain parallel code paths per version.
2. **Testing matrix.** N versions × M features = exponential test combinations.
3. **Documentation burden.** Each version needs its own documentation, examples, and SDK support.
4. **Bug fixes propagate slowly.** A fix in v3 must also be applied to v1 and v2 if those versions are still supported.
5. **Cognitive load.** New engineers must understand which version they're working on and which version-specific quirks exist.

**Mitigation strategies:**

- **Adaptation layers.** Keep one canonical internal API and write thin adapters that translate old versions to the current shape. The old versions become shells; all real logic lives in one place.
- **Aggressive migration assistance.** Provide automated migration tools, clear guides, and personal support for consumers still on old versions.
- **Sunset aggressively but fairly.** Announce early, warn loudly, sunset on schedule. Don't let old versions linger indefinitely.
- **Limit concurrent versions.** Support at most 2–3 versions at a time. When v3 launches, v1 enters sunset.

## Build It

### Step 1: Minimal Version — Document a Versioning Policy

Write a `VERSIONING.md` for a hypothetical API:

```markdown
# API Versioning Policy

## Current Version: v2

## Versioning Strategy: URL Path

All endpoints are prefixed with `/v{major}/`.

## What Constitutes a Breaking Change

- Removing a field from a response
- Changing a field's type
- Removing an endpoint
- Renaming a field
- Changing field semantics
- Adding a required request field

## What Constitutes a Compatible Change

- Adding a new optional response field
- Adding a new endpoint
- Adding a new optional query parameter

## Deprecation Process

1. Announce in changelog and API docs with a sunset date (min 6 months).
2. Add `Deprecation` and `Sunset` headers to responses.
3. After sunset date, return `410 Gone` for deprecated endpoints.

## Supported Versions

- v2 (current)
- v1 (deprecated; sunset: 2025-12-01)
```

### Step 2: Realistic Version — Full Versioning with Expand/Contract

Add a database migration plan and contract test:

```markdown
# Migrating: `name` → `first_name` + `last_name`

## Expand Phase (Week 1)
- ALTER TABLE users ADD COLUMN first_name VARCHAR(100);
- ALTER TABLE users ADD COLUMN last_name VARCHAR(100);
- Update API code to write to both `name` and `first_name`/`last_name`.
- v2 API returns `first_name` and `last_name`; v1 API still returns `name`.

## Migrate Phase (Week 2–4)
- Backfill: UPDATE users SET first_name = split_part(name, ' ', 1), last_name = split_part(name, ' ', 2) WHERE first_name IS NULL;
- Monitor v1 traffic. Announce deprecation.

## Contract Phase (Week 5)
- Once v1 traffic is zero, stop writing `name`.
- ALTER TABLE users DROP COLUMN name;
- Remove v1 API route.
```

## Use It

**Stripe's versioning system** is worth reading in detail. Their approach is documented at https://stripe.com/docs/api/versioning. Key observations:

- Stripe uses date-based versioning via a header (`Stripe-Version`).
- Each version is a snapshot of all changes made before that date.
- Breaking changes result in a new version date; additive changes apply to all versions.
- They maintain an internal changelog and can map any date version to a set of behavior changes.

Compare your hand-built versioning policy against Stripe's. The differences that matter:

1. Stripe doesn't support multiple URL paths; one URL, one resource. Versioning is in content negotiation.
2. Stripe's default is the version you signed up with, not the latest. This is a crucial design choice—it means your integration never breaks unless you explicitly upgrade.

## Read the Source

- **Kubernetes API machinery:** `staging/src/k8s.io/apimachinery/pkg/runtime/` in the Kubernetes repo. Look at how API group versions are registered and how conversion between versions works.
- **Pact specification:** https://github.com/pact-foundation/pact-specification — the contract testing specification that defines how consumer and provider contracts are structured.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`versioning_reference.md`** — A quick-reference card covering versioning strategies, deprecation checklists, and compatibility rules.

## Exercises

1. **Easy** — Write a compatibility analysis for this change: "Add a `phone` field to the `POST /users` response." Is it breaking or compatible? Explain why.
2. **Medium** — Design an expand/contract migration plan for changing a `status` field from string (`"active"`, `"inactive"`) to an enum with three values (`"active"`, `"inactive"`, `"suspended"`). Write out the three phases.
3. **Hard** — Implement a contract test (using Pact or a similar tool) for a hypothetical provider-consumer pair. The contract should catch a breaking change where the provider removes a field the consumer depends on. Show the contract, the provider verification, and the CI failure message when the breaking change is introduced.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SemVer | "Just version it" | A contract: major = breaking, minor = additive, patch = fix |
| Breaking change | "We changed the API" | Any change that could cause an existing consumer to fail |
| Backward-compatible | "It still works" | Every existing consumer keeps working without changes |
| Deprecation | "It's going away" | Feature still works but will be removed; stop using it |
| Sunset | "It's dead" | The date after which the deprecated feature stops working |
| Expand/Contract | "The migration pattern" | Add new → migrate data → remove old; never be in a broken state |
| Contract test | "Pact test" | A test that verifies the provider still satisfies the consumer's expectations |
| Content negotiation | "Accept header versioning" | Using HTTP Accept headers to request a specific API version |

## Further Reading

- **Semantic Versioning 2.0.0** — https://semver.org — the canonical specification.
- **RFC 8594: The Sunset HTTP Header Field** — https://datatracker.ietf.org/doc/html/rfc8594 — the standard for sunset headers.
- **RFC 5741: AtomPub and Deprecation headers** — historical context on HTTP deprecation.
- **Pact.io** — https://pact.io — consumer-driven contract testing framework.
- **Stripe API Versioning** — https://stripe.com/docs/api/versioning — production example of date-based header versioning.
- **Google API Improvement Proposals** — https://aip.dev/ — design guidelines for versioning and compatibility in long-lived APIs.
- **Kubernetes API Conventions** — https://kubernetes.io/docs/reference/using-api/api-concepts/ — how k8s handles versioning at scale.