# Output Artifact: Property Design Checklist

Use this checklist when adding property-based tests:

1. Define invariants independent from implementation details.
2. Add at least one algebraic property (idempotence, associativity, round-trip).
3. Design generators for both typical and edge-case distributions.
4. Ensure failing inputs are reproducible and shrunk in CI logs.
5. Pair properties with a few fixed regression examples.
