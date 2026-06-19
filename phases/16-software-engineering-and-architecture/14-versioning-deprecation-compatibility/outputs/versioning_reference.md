# Versioning, Deprecation & Compatibility — Quick Reference

## Versioning Strategies at a Glance

| Strategy | Example | Pros | Cons | Used By |
|----------|---------|------|------|---------|
| URL Path | `/v2/users` | Visible, cacheable, easy to debug | Breaks REST resource identity, multiple code paths | Twilio, SendGrid |
| Query Param | `/users?v=2` | URL stays same | Easy to miss, caching headaches | Google Data API |
| Custom Header | `X-API-Version: 2` | Clean URLs, flexible | Invisible in logs, needs custom logging | Stripe (date-based) |
| Content Negotiation | `Accept: application/vnd.api.v2+json` | Most REST-correct, HTTP-standard | Complex, unfamiliar to many devs | GitHub |

## Compatibility Rules

### Safe Changes (no version bump needed)

- Adding an optional response field
- Adding a new endpoint
- Adding an optional request parameter (with default)
- Widening a type (`int` → `int | float`)
- Adding an enum value (if consumers tolerate unknowns)
- Relaxing a constraint (increasing a max page size limit)

### Breaking Changes (require major version bump)

- Removing a response field
- Renaming a field
- Changing a field's type
- Changing field semantics (same name, different meaning)
- Making an optional field required
- Adding a required request field
- Removing an endpoint
- Tightening a constraint (reducing limits, adding validation)
- Changing error response format or codes

### Gray Area (depends on consumer behavior)

- Adding an enum value (breaks exhaustive switch statements)
- Changing field order in JSON (breaks positional parsers)
- Changing internal identifiers (breaks consumers that cache them)

## Semantic Versioning Cheat Sheet

```
MAJOR.MINOR.PATCH (e.g., 2.4.1)

MAJOR → Breaking API changes (consumers MUST update)
MINOR → New backward-compatible features (consumers CAN update safely)
PATCH → Backward-compatible bug fixes (consumers SHOULD update freely)

Given version A.B.C, upgrading to A.B'.C' (B' >= B) is safe within the same MAJOR version.
A MAJOR bump means "something broke."
```

## Deprecation Checklist

### Phase 1: Announce

- [ ] Identify all consumers (analytics, contract tests, logs)
- [ ] Set a concrete sunset date (min 6 months for external consumers)
- [ ] Write migration documentation with before/after examples
- [ ] Publish in changelog, API docs, and notify consumers directly

### Phase 2: Warn

- [ ] Add `Deprecation: true` response header
- [ ] Add `Sunset: <date>` response header
- [ ] Add `Link: <migration-docs-url>; rel="deprecation"` header
- [ ] Log warnings server-side with consumer identification
- [ ] Emit metrics: daily count of consumers hitting deprecated features
- [ ] Provide migration scripts or tools if the change is mechanical

### Phase 3: Sunset

- [ ] Verify zero traffic on deprecated feature
- [ ] If traffic > 0: extend the sunset period; do not break consumers on schedule
- [ ] Remove deprecated code and documentation
- [ ] For removed endpoints: return `410 Gone` for 1–3 months

### Phase 4: Post-Mortem

- [ ] Document what worked and what didn't
- [ ] Update deprecation playbooks based on lessons learned
- [ ] Clean up feature flags and routing logic

## Expand/Contract Migration Pattern

```
Phase 1 — EXPAND: Add new alongside old
  ALTER TABLE users ADD COLUMN full_name VARCHAR(200);
  -- App writes to BOTH `name` and `full_name`
  -- v1 API: {name: "..."}         → still works
  -- v2 API: {name: "...", full_name: "..."} → new format

Phase 2 — MIGRATE: Move consumers to new
  UPDATE users SET full_name = name WHERE full_name IS NULL;
  -- Monitor v1 traffic; deprecate v1
  -- All reads shift to `full_name`

Phase 3 — CONTRACT: Remove old
  -- After v1 traffic → zero
  ALTER TABLE users DROP COLUMN name;
  -- Remove v1 API route
```

**Rule:** At every point in time, the system works. No migration window where things are broken.

## Deprecation Period Guidelines

| Consumer Type | Minimum Period | Rationale |
|---------------|----------------|-----------|
| Internal services | 2–4 weeks | You control deployments |
| Public web/mobile clients | 3–6 months | Users may not update |
| Third-party integrations | 6–12 months | You don't control their roadmap |
| Open-source libraries (SemVer) | Until next major version | SemVer contract |

## Feature Flag Rollout Pattern

```
Deploy v2 behind flag → Enable for 1 consumer → Verify → 10% → 50% → 100% → Remove v1 and flag
```

Flags are a tool, not a lifestyle. Remove the flag once migration is complete. A permanent flag is technical debt.

## Contract Testing (Pact) — Quick Start

1. **Consumer writes contract**: "When I call GET /users/1, I expect {id: 1, name: 'Ada'}"
2. **Contract published to broker**: Shared between consumer and provider CI pipelines
3. **Provider verifies on every build**: Runs contract against provider's latest code
4. **Breaking change caught in CI**: Provider's build fails before the change deploys

```
Consumer CI:  Write contract → Publish to broker
Provider CI:  Fetch contracts → Verify → Pass/Fail
```

## Real-World Versioning Approaches

| Company | Strategy | Version Format | Key Trait |
|---------|----------|----------------|-----------|
| Stripe | Header | `2024-01-15` | Date-based; defaults to signup version |
| GitHub | Content Negotiation | `v3` | Media type in Accept header |
| Kubernetes | URL Path + Groups | `v1`, `v1beta1` | Per-API-group versioning |
| Twilio | URL Path | `2010-04-01` | Date-based path; decade-old versions still running |

## Cost of Maintaining Multiple Versions

- **Code complexity**: Each version is a code path to maintain
- **Testing matrix**: N versions × M features
- **Documentation**: Separate docs per version
- **Bug fix propagation**: Same fix must be applied to each supported version
- **Cognitive load**: Engineers must track which quirks belong to which version

**Mitigation**: Use adaptation layers (one canonical API, thin version shims), limit to 2–3 concurrent versions, sunset aggressively but fairly.