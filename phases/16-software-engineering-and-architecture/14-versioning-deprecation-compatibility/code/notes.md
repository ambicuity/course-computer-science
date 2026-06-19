# Notes — Versioning, Deprecation, Compatibility

## Versioning Strategy Comparison

| Strategy | Mechanism | Visibility | REST Purity | Caching | Complexity | Best For |
|----------|-----------|------------|-------------|---------|------------|----------|
| URL Path | `/v1/users`, `/v2/users` | High (in URL) | Low (different URLs = different resources) | High (URL cacheable) | Low | Public APIs, easy debugging |
| Query Param | `/users?v=1` | Medium | Medium | Low (same URL, varying response) | Low | Quick-and-dirty versioning |
| Custom Header | `X-API-Version: 2` | Low (invisible in logs) | High (same resource) | Medium (needs Vary header) | Medium | Internal APIs, controlled clients |
| Content Negotiation | `Accept: application/vnd.api.v2+json` | Low | Highest (HTTP-standard) | Medium (needs Vary header) | High | APIs prioritizing REST correctness |

### Decision Matrix

```
Is this a public API?
├── Yes → Prioritize visibility → URL Path or Content Negotiation
│   ├── Consumers vary widely? → URL Path (simplest to adopt)
│   └── Consumers are sophisticated? → Content Negotiation (cleanest)
└── No (internal) → Prioritize flexibility → Custom Header or Content Negotiation
    ├── You control all clients? → Custom Header (easy to implement)
    └── Multiple teams with standards? → Content Negotiation (formal)
```

## Deprecation Checklist

### Before Deprecating

- [ ] Identify all consumers of the deprecated feature (check API analytics, contract tests, logs)
- [ ] Determine the minimum deprecation period based on slowest consumer's update cycle
- [ ] Design the migration path: what do consumers need to change?
- [ ] Write migration documentation with before/after examples
- [ ] Set a concrete sunset date (not "TBD" or "eventually")

### During Deprecation

- [ ] Add `Deprecation: true` header to all affected responses
- [ ] Add `Sunset: <date>` header with the removal date
- [ ] Add `Link: <migration-docs-url>; rel="deprecation"` header
- [ ] Log deprecation warnings server-side with consumer identification
- [ ] Emit metrics: count of consumers still using the deprecated feature per day
- [ ] Notify consumers via email, changelog, and Slack/Discord
- [ ] Provide a migration tool or script if the change is mechanical

### At Sunset

- [ ] Verify zero traffic on deprecated feature via metrics
- [ ] If traffic > 0: extend the period; do NOT break consumers on schedule
- [ ] Remove deprecated code paths
- [ ] Remove deprecated documentation
- [ ] Remove feature flags or routing logic for the old version
- [ ] Update API version if using SemVer (bump major if the removal is breaking)
- [ ] Announce removal in changelog

### After Sunset

- [ ] For removed endpoints: return `410 Gone` for a transition period (1–3 months)
- [ ] Monitor error rates for consumers that missed the migration
- [ ] Conduct a post-mortem: what worked, what didn't, how to deprecate better next time

## Migration Patterns

### Pattern 1: Expand/Contract (Parallel Change)

**Use when:** Renaming, splitting, or restructuring a field/table.

```
Phase 1 (Expand): Add new alongside old
  DB:  ALTER TABLE users ADD COLUMN full_name;
  API: v1 returns {name}, v2 returns {name, full_name}
  App: Write to both `name` and `full_name`

Phase 2 (Migrate): Move consumers to new; backfill data
  DB:  UPDATE users SET full_name = name WHERE full_name IS NULL;
  API: Both v1 and v2 active; v1 deprecated
  App: All new reads use `full_name`

Phase 3 (Contract): Remove old
  DB:  ALTER TABLE users DROP COLUMN name;
  API: Remove v1 endpoint
  App: Only `full_name` exists
```

**Rule:** At every point in time, the system works. There is no "migration window" where things are broken.

### Pattern 2: Version Shim (Adaptation Layer)

**Use when:** Multiple API versions must coexist long-term.

```
Client → v1 endpoint → Shim layer → Internal canonical API
Client → v2 endpoint → Shim layer → Internal canonical API
```

- The shim translates between the external version's format and the internal format.
- All business logic lives once, in the canonical API.
- Shims are thin: just field mapping, type coercion, and default values.
- When v1 is sunset, you delete the v1 shim. No business logic is lost.

### Pattern 3: Feature Flag Cutover

**Use when:** You want to test a new version with specific consumers before full rollout.

```
Request → Feature flag evaluation
  ├── Consumer A → v2 handler (new)
  ├── Consumer B → v2 handler (new)
  └── All others → v1 handler (current)
```

Steps:
1. Deploy v2 behind a flag. Enable for 1 internal consumer.
2. Verify: correctness, latency, error rate.
3. Enable for more consumers incrementally (1% → 10% → 50% → 100%).
4. Once 100% traffic on v2, remove v1 and the flag.

### Pattern 3.5: Dual-Write with Consistency Check

**Use when:** Migrating to a new data store or schema where you need to verify consistency.

```
Write path:
  1. Write to old store
  2. Write to new store
  3. Async: compare write results, alert on mismatch

Read path:
  Phase 1: Read from old store (trusted)
  Phase 2: Read from new store, shadow-compare with old, alert on mismatch
  Phase 3: Read from new store (now trusted)
```

### Compatibility Rules Quick Reference

```
SAFE (backward-compatible, no version bump needed):
  ✅ Add optional response field
  ✅ Add new endpoint
  ✅ Add optional request parameter (with default)
  ✅ Widen a type (int → float, string → enum + string)
  ✅ Add enum value (if consumers handle unknown values)
  ✅ Increase rate limit / page size limit

BREAKING (requires major version bump or parallel version):
  ❌ Remove response field
  ❌ Rename response field
  ❌ Change field type (string → int)
  ❌ Change field semantics (same value, different meaning)
  ❌ Make optional field required
  ❌ Add required request field
  ❌ Remove endpoint
  ❌ Change error format/codes
  ❌ Tighten constraints (reduce page size, add validation)
  ❌ Reorder protobuf fields

GRAY AREA (depends on consumer behavior):
  ⚠️  Add enum value (consumers using switch/default are fine; those using exhaustive if/else may break)
  ⚠️  Change field order in JSON (most JSON parsers don't care, but some consumers may use positional parsing)
  ⚠️  Change internal IDs (if consumers store or log them)
```