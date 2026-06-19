# Lesson Template

Use this template when authoring a lesson body in `course-computer-science`. The
scaffolding script (`scripts/scaffold_course.py`) seeds every lesson with this
skeleton already filled in — your job is to replace the `[...]` placeholders
with real content.

## Folder Structure

```
NN-lesson-name/
├── code/
│   ├── main.c          (C — for memory, OS, networks, kernel-adjacent topics)
│   ├── main.rs         (Rust — modern systems, safety-critical, lock-free)
│   ├── main.cpp        (C++ — graphics, low-latency)
│   ├── main.py         (Python — algorithms, theory, prototypes, glue)
│   ├── main.s          (RISC-V assembly — Phase 06 mostly)
│   ├── main.go         (Go — concurrency, distributed)
│   ├── Main.hs         (Haskell — type theory, paradigms)
│   ├── main.sql        (SQL — Phase 10 throughout)
│   ├── Main.tla        (TLA+ — Phase 17 formal models)
│   └── main.ts         (TypeScript — Phase 16 architecture, web)
├── docs/
│   └── en.md           (lesson narrative)
├── quiz.json           (pre/post MCQs with explanations)
└── outputs/
    └── (the reusable artifact this lesson ships — CLI, library, etc.)
```

Pick **2–4 languages per lesson** by intent. Never use all of them.

## Documentation Format (`docs/en.md`)

```markdown
# [Lesson Title]

> [One-line motto — the core idea that sticks]

**Type:** Build | Learn
**Languages:** [list what's used]
**Prerequisites:** [Phase / lesson references]
**Time:** ~[estimated time] minutes

## Learning Objectives

- [Specific, testable outcome 1]
- [Specific, testable outcome 2]
- [Specific, testable outcome 3]

## The Problem

[2–3 paragraphs. What can't you do without this? Make it concrete — show a
scenario where not knowing this hurts. Tie it to the phase capstone.]

## The Concept

[Diagrams and intuition. No code yet. ASCII diagrams, tables, or Mermaid.
Build mental models before implementation.]

## Build It

[Step-by-step implementation from scratch. Start with the simplest version,
then add complexity. Every code block should be runnable on its own.]

### Step 1: [Name]

[Explanation.]

    [code block]

### Step 2: [Name]

[Explanation.]

    [code block]

[...continue...]

## Use It

[Now show how the production tool / library / system does this. For CS:
- Systems: link to the relevant Linux / PostgreSQL / etcd / LLVM source file.
- Algorithms: show the language's standard library version.
- Protocols: cite the RFC section.
Compare the production version against yours — what does it do that yours
doesn't, and why?]

## Read the Source

- [file path in a production codebase + 1–2 line note on what to look at]

## Ship It

[What reusable artifact does this lesson produce? CLI tool, library, parser,
protocol implementation, kernel module, etc. Save it in `outputs/`.]

## Exercises

1. [Easy — reinforce the core concept]
2. [Medium — apply it to a different problem]
3. [Hard — extend or combine with prior lessons]

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| [term] | [common misconception] | [actual definition] |

## Further Reading

- [Resource 1](url) — [why it's worth reading]
- [Resource 2](url) — [why it's worth reading]
```

## Code File Guidelines

- Code must build and run without errors.
- No comments unless they explain a non-obvious WHY (a constraint, an
  invariant, a workaround). Don't narrate WHAT the code does.
- Pick the language that fits the topic. Don't write a kernel allocator in
  Python and don't write graph theory proofs in C.
- Start simple; build up complexity in clearly named steps.
- Include `Cargo.toml`, `requirements.txt`, or a one-line `make` rule if the
  lesson needs dependencies.

## Quiz Format (`quiz.json`)

```json
{
  "questions": [
    {
      "stage": "pre",
      "question": "Question text here.",
      "options": ["A", "B", "C", "D"],
      "correct": 1,
      "explanation": "Why B is right and the others aren't."
    },
    {
      "stage": "post",
      "question": "Harder question that applies the lesson.",
      "options": ["A", "B", "C", "D"],
      "correct": 2,
      "explanation": "Why C is right."
    }
  ]
}
```

- 2–6 questions per lesson, split between `"pre"` and `"post"`.
- `correct` is the 0-based index into `options`.
- Explanations are mandatory and should teach, not just confirm.

## Output File Format (`outputs/`)

The CS course's reusable artifacts are **tools, libraries, protocol
implementations, parsers, kernel modules, or visualizations** — not prompts.
Conventions:

- Code artifacts: drop a working subdirectory (e.g., `outputs/bptree/` with its
  own `Cargo.toml` or `Makefile`).
- Standalone scripts: name them descriptively (e.g., `outputs/raft-trace.py`).
- README inside `outputs/` explaining how to run the artifact and where it
  reappears in later phases (e.g., the B+-tree from Phase 03 is reused by the
  database engine in Phase 10).
