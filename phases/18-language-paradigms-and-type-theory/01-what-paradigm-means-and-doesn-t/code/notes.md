# Paradigm Selection Notes

## Quick rubric

1. Is state mutation central and unavoidable?
2. Are transformations naturally compositional?
3. Is behavior best modeled around entities with lifecycles?
4. Are constraints more natural than explicit control flow?

## Pragmatic mapping

- High data transformation: functional-first.
- Rich mutable domain objects: OO-heavy.
- Rule engines/satisfiability: declarative/logic.
- System scripting/orchestration: imperative baseline with targeted FP helpers.

## Failure modes

- Forcing pure FP where side effects dominate.
- Heavy OO hierarchy for simple data transforms.
- Over-generalized abstractions before requirements stabilize.
