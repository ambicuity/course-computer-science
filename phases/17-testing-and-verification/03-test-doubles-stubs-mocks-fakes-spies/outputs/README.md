# Output Artifact: Test Double Selection Rubric

Use this rubric in PR reviews:

1. Is the claim behavioral or interaction-protocol specific?
2. Can a stub/fake satisfy determinism with lower coupling?
3. Is a spy enough for side-effect assertions?
4. If using a strict mock, is the interaction itself contract-critical?

This keeps tests resilient while preserving necessary contract checks.
