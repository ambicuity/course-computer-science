# Verified Software Notes

## seL4 highlights

- Mechanized proof of key kernel properties.
- Strong assurance claims for isolation and correctness assumptions.

## CompCert highlights

- Verified C compiler with proven semantic preservation for supported subset.
- Reduces compiler-induced miscompilation risk.

## Practical rubric

1. Is failure impact high?
2. Is behavior specifiable precisely?
3. Is interface stable enough for proof maintenance?
4. Is there available formal-method expertise?
5. Can assumptions at boundaries be documented and monitored?

## Decision categories

- Verify now: high-impact stable core.
- Verify later: important but fast-evolving modules.
- Test only: low-impact rapidly changing features.
