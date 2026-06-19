# Mermaid Cheat Sheet — Copy-Paste Skeletons

GitHub, GitLab, VS Code Markdown preview, mkdocs-material, Obsidian — all render these natively.

## Flowchart

```mermaid
flowchart TB
  A[Start] --> B{Decision?}
  B -->|yes| C[Action 1]
  B -->|no|  D[Action 2]
  C --> E[End]
  D --> E
```

Direction: `TB` (top→bottom), `BT`, `LR` (left→right), `RL`.

## Sequence

```mermaid
sequenceDiagram
  autonumber
  actor Client
  participant API as API server
  participant DB

  Client->>API: POST /order
  API->>DB: BEGIN; INSERT
  DB-->>API: ok
  API-->>Client: 201 Created
  Note over API,DB: async commit hook fires
```

## State

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Loading: fetch()
  Loading --> Ready: data received
  Loading --> Error: failure
  Error --> Idle: retry
  Ready --> [*]
```

## Class

```mermaid
classDiagram
  class Shape {
    <<abstract>>
    +area()* float
  }
  class Circle {
    -r: float
    +area() float
  }
  class Square {
    -s: float
    +area() float
  }
  Shape <|-- Circle
  Shape <|-- Square
```

## Entity-relationship

```mermaid
erDiagram
  USER ||--o{ ORDER : places
  ORDER ||--|{ LINE_ITEM : contains
  PRODUCT ||--o{ LINE_ITEM : "appears in"

  USER {
    int     id PK
    string  email
  }
  ORDER {
    int     id PK
    int     user_id FK
    date    created_at
  }
```

## Pie

```mermaid
pie title CPU breakdown
  "user"   : 60
  "sys"    : 15
  "iowait" : 5
  "idle"   : 20
```

## Gantt

```mermaid
gantt
  title Phase 0 schedule
  dateFormat YYYY-MM-DD
  section Foundations
  Toolchain         :done,    a1, 2026-05-12, 2d
  Terminal & Shell  :active,  a2, after a1,    1d
  Git Deep          :         a3, after a2,    2d
```

## Tips

- IDs (`A`, `B`) are case-sensitive and reused across blocks.
- Quote labels with `["text"]` if they contain `:`, `(`, or spaces with weird chars.
- For long subtitles, use `\n` (literal `\n` in the source); Mermaid renders it as a newline.
- `subgraph name [...]` groups nodes.
