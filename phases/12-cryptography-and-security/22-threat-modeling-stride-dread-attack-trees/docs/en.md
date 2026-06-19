# Threat Modeling — STRIDE, DREAD, attack trees

> Threat Modeling — STRIDE, DREAD, attack trees — the part of CS you can't skip.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 12 lessons 01–21
**Time:** ~60 minutes

## Learning Objectives

- Explain why reactive security (fixing what attackers find) is insufficient, and how threat modeling shifts security left to the design phase.
- Describe the six STRIDE threat categories and map each to the security property it violates (authentication, integrity, non-repudiation, confidentiality, availability, authorization).
- Apply STRIDE per-element analysis to each component of a data flow diagram — external entities, processes, data stores, and data flows.
- Score threats using the DREAD risk-rating model (Damage, Reproducibility, Exploitability, Affected Users, Discoverability) to produce a numeric priority ranking.
- Build an attack tree from an attacker's goal, using OR and AND nodes to enumerate attack paths, and prune infeasible branches by assigning cost/difficulty/skill to each leaf.
- Construct a data flow diagram (DFD) with trust boundaries for a simple web application and identify threats at each boundary crossing.

## The Problem

You cannot fix every security bug. There are too many, the threat landscape shifts constantly, and engineering time is finite. Without a systematic way to decide which bugs matter, teams fall into reactive security: they fix whatever the latest scanner flags, whichever CVE has the highest CVSS score today, or whatever an attacker happened to find first. This is not strategy — it is whack-a-mole.

Consider a typical web application with hundreds of dependencies, dozens of endpoints, and a half-dozen data stores. A vulnerability scanner might report 80 findings: some are SQL injection in a critical auth endpoint (game-over), some are reflective XSS in a rarely-used admin page (annoying but contained), and some are false positives from a library version check. Which do you fix first? How do you explain your priority order to a security auditor or a regulator?

**Threat modeling** answers these questions before a single line of code is written. It is a structured process — performed during the design phase — that identifies what you are building, where an attacker might strike, how much damage each strike could cause, and which defenses are worth building. It formalizes the question: "What are the worst things that could happen to this system, and what are we going to do about them?"

The frameworks you will learn in this lesson — STRIDE, DREAD, and attack trees — are the three most widely used tools for answering that question. STRIDE tells you *what kind* of threat you face. DREAD tells you *how bad* it is. Attack trees tell you *how* an attacker could get there. Together, they replace guesswork with a repeatable methodology.

## The Concept

### The Three Pillars of Threat Modeling

Threat modeling rests on three intellectual frameworks, each answering a different question:

| Framework | Question | Output |
|-----------|----------|--------|
| **STRIDE** | What kind of threat is this? | A categorized list of threats per system element |
| **DREAD** | How risky is this threat? | A numeric score for prioritization |
| **Attack Trees** | How would an attacker accomplish this? | A hierarchical diagram of attack paths |

### 1. STRIDE — Categorizing Threats

STRIDE was developed at Microsoft in the late 1990s by Loren Kohnfelder and Praerit Garg as part of the Security Development Lifecycle (SDL). It decomposes security into six threat categories, each corresponding to a specific security property:

| Letter | Threat | Security Property Violated | Simple Definition |
|--------|--------|---------------------------|-------------------|
| **S** | Spoofing | Authentication | Impersonating someone or something |
| **T** | Tampering | Integrity | Modifying data without authorization |
| **R** | Repudiation | Non-repudiation | Denying that an action was performed |
| **I** | Information Disclosure | Confidentiality | Exposing data to unauthorized parties |
| **D** | Denial of Service | Availability | Disrupting service to legitimate users |
| **E** | Elevation of Privilege | Authorization | Gaining unauthorized access or permissions |

Each threat type maps to a security requirement and a control:

| Threat | Security Requirement | Example Control |
|--------|---------------------|-----------------|
| Spoofing | Authentication | MFA, client certificates, OAuth |
| Tampering | Integrity | Digital signatures, checksums, HMAC |
| Repudiation | Non-repudiation | Audit logs, signing, blockchain |
| Information Disclosure | Confidentiality | Encryption at rest and in transit, access control |
| Denial of Service | Availability | Rate limiting, load balancing, auto-scaling |
| Elevation of Privilege | Authorization | RBAC, least privilege, principle of least authority |

**Real-world examples (CVEs):**

- **Spoofing (CVE-2023-23397):** Microsoft Outlook Elevation of Privilege vulnerability. An attacker could send a specially crafted email that triggers an NTLM credential leak — the attacker spoofs a trusted sender and steals authentication material.
- **Tampering (CVE-2017-5638):** Apache Struts2 vulnerability allowing an attacker to tamper with file upload parameters in the `Content-Type` header, leading to remote code execution. This was the vulnerability exploited in the Equifax breach.
- **Repudiation (CVE-2014-0160, Heartbleed):** While primarily an information disclosure, Heartbleed also enabled repudiation — an attacker could extract server memory without leaving traces in the server's logs, making it impossible for the server operator to prove the attack occurred.
- **Information Disclosure (CVE-2019-19781):** Citrix ADC vulnerability allowing unauthenticated attackers to read arbitrary files from the server, including configuration files with credentials.
- **Denial of Service (CVE-2022-0847, DirtyPipe):** While primarily an EoP, DirtyPipe could be used to overwrite read-only files including `/etc/passwd`, effectively enabling DoS by rendering the system non-functional.
- **Elevation of Privilege (CVE-2021-44228, Log4Shell):** A crafted log message triggers JNDI lookup loading remote code — the attacker goes from arbitrary log input to full remote code execution on the server.

#### Per-Element STRIDE Analysis

In practice, STRIDE is applied **per element** of a data flow diagram. Not every threat type applies to every element type:

| Element Type | Spoofing | Tampering | Repudiation | Info Disclosure | DoS | EoP |
|-------------|----------|-----------|-------------|-----------------|-----|-----|
| **External Entity** (user, third-party system) | Authentication — must prove identity | N/A — entity doesn't process data | Non-repudiation — need signing | N/A — entity doesn't store data | N/A | N/A |
| **Process** (web server, worker) | Authentication — identify caller | Integrity — protect code/data | Signing — log actions | Confidentiality — encrypt output | Availability — resource limits | Authorization — restrict actions |
| **Data Store** (database, file system) | Authentication — who accesses it | Integrity — detect modification | Signing — audit trail | Confidentiality — encrypt data | Availability — redundant storage | N/A — data stores don't execute code |
| **Data Flow** (HTTPS, SQL query) | Authentication — verify endpoints | Integrity — detect tampering in transit | Signing — prove origin | Confidentiality — encrypt channel | Availability — maintain connection | N/A — flows don't change privilege |

The key insight: by enumerating every element type in your DFD and asking "what could go wrong?" for each STRIDE category that applies to that type, you systematically cover the entire attack surface.

### 2. DREAD — Rating Risk

Once you have a list of threats (from STRIDE), you need to decide which to fix first. DREAD provides a numeric risk score. Each category is scored **1–3**:

| Category | What it measures | 1 (Low) | 2 (Medium) | 3 (High) |
|----------|-----------------|---------|-----------|---------|
| **D**amage Potential | How bad is the impact? | Limited data exposure | Major data loss or service degradation | Full system compromise or data destruction |
| **R**eproducibility | Can the attacker reproduce it reliably? | Very hard, race condition dependent | Complex multi-step attack | One click, always works, repeatable |
| **E**xploitability | How hard is it to exploit? | Requires local access or extensive resources | Requires network access and moderate skill | Unauthenticated, simple tooling, low skill |
| **A**ffected Users | How many users are affected? | None or single user | Subset of users | All users (anonymous internet) |
| **D**iscoverability | How easy is it to find? | Requires source code access | Can be found with scanning tools | Publicly known vulnerability |

The total score is **Damage + Reproducibility + Exploitability + Affected Users + Discoverability** (range: 5–15). Typical thresholds:

| Score | Priority | Action |
|-------|----------|--------|
| 12–15 | CRITICAL | Fix immediately, block release |
| 9–11 | HIGH | Fix in current sprint |
| 5–8 | MEDIUM | Fix in next iteration |
| Below 5 | LOW | Accept risk or backlog |

**Criticism of DREAD:**

Microsoft deprecated DREAD in the early 2010s in favor of STRIDE per-element analysis. The core criticism: DREAD scores are **subjective**. Two teams evaluating the same threat often arrive at different scores. Discoverability is especially contentious — it conflates "how easy is it for an attacker to find" with "how well known is this vulnerability class." A vulnerability that is trivially discoverable by someone who knows where to look (e.g., a debug endpoint) scores differently than one that requires reverse engineering of a closed-source binary.

Despite this, DREAD remains widely used for two reasons: it produces a **single number** that non-security stakeholders can understand, and it forces teams to discuss *why* a threat ranks where it does, which is often more valuable than the number itself.

### 3. Attack Trees — Modeling Attack Paths

Attack trees were introduced by Bruce Schneier in his 1999 Dr. Dobb's Journal article and later expanded in "Secrets & Lies" (2000). An attack tree is a hierarchical diagram:

- **Root node:** The attacker's ultimate goal (e.g., "Steal customer database").
- **Child nodes:** Ways to achieve the parent goal.
  - **OR nodes:** Any one child is sufficient (the attacker needs only one path).
  - **AND nodes:** All children must be satisfied simultaneously.
- **Leaf nodes:** Specific actions or conditions (the concrete attack steps).

```
Goal: Steal customer database
├── OR: SQL injection on web endpoint
│   ├── AND: Find injectable parameter
│   └── AND: Bypass WAF/input filter
├── OR: Steal encrypted backup tape
│   ├── AND: Physical access to datacenter
│   └── AND: Obtain encryption key
├── OR: Extract from application memory
│   ├── AND: RCE via deserialization vulnerability
│   └── AND: Dump heap containing credentials
└── OR: Bribe DBA
    ├── AND: Identify DBA
    └── AND: Social-engineer or pay for credentials
```

**Assigning properties to nodes:**

Each node can carry attributes:
- **Cost:** Dollars, compute resources, or time required.
- **Difficulty:** Skill level needed (novice, script kiddie, expert, nation-state).
- **Detection probability:** Likelihood the action triggers an alarm.
- **Legal risk:** Severity of legal consequences if caught.

These attributes let you **prune** infeasible paths. A nation-state attacker might accept high cost and low detectability; a script kiddie will not. By pruning, you identify which paths are actually live threats.

**AND vs OR semantics:**

Attack trees use standard logic:
- **OR** = disjunction. The parent is achievable if *any* child is achievable. (Most tree nodes are OR by default.)
- **AND** = conjunction. The parent is achievable only if *all* children are achievable. AND nodes typically represent sub-goals that must be executed in parallel or sequence without failure at any step.

A node without children is a **leaf** — a primitive action the attacker can take directly.

### Data Flow Diagrams (DFDs) for Threat Modeling

Before you can apply STRIDE, you need a map of the system. **Data Flow Diagrams** provide that map. DFDs originated in structured systems analysis (Yourdon/DeMarco, 1970s) and were adapted for threat modeling by Microsoft's SDL.

**DFD elements and notation:**

| Element | Shape | Example |
|---------|-------|---------|
| **External Entity** | Rectangle | User, third-party API |
| **Process** | Circle / Rounded rectangle | Web server, worker, cron job |
| **Data Store** | Two parallel lines (or open rectangle) | Database, cache, filesystem |
| **Data Flow** | Arrow | HTTPS request, SQL query, RPC call |
| **Trust Boundary** | Dashed line | Separates internet from internal network, or web tier from data tier |

**The threat modeling DFD process:**

1. **Draw the system:** Identify all external entities, processes, data stores, and data flows.
2. **Draw trust boundaries:** Every dashed line is a place where security properties change — data crossing a boundary must be protected differently on each side.
3. **Apply STRIDE per element:** For each element on the DFD, run the STRIDE per-element matrix.
4. **Document threats:** For each element + threat type combination that could realistically occur, write a brief threat description.

**Trust boundaries are the most critical part.** Every time data crosses a trust boundary, it changes from "data I control" to "data I don't control" (or vice versa). Common trust boundaries:
- Internet ↔ Internal network (the classic boundary)
- Web tier ↔ Database tier (different access models)
- User ↔ Application (user input must never be trusted)
- Container ↔ Host (container escape scenarios)

### Security Requirements vs Mitigations

A common confusion in threat modeling: what is a *requirement* versus what is a *mitigation*?

| | Definition | Example |
|------------|------------|---------|
| **Security Requirement** | What the system must guarantee | "The system must authenticate all users before allowing access to any endpoint" |
| **Mitigation (Control)** | How the requirement is met | "Use OAuth 2.0 with PKCE and enforce MFA for admin accounts" |

STRIDE threats → Security requirements → Mitigations:

| Threat | Security Requirement | Possible Mitigations |
|--------|---------------------|---------------------|
| Spoofing | Authentication | OAuth, SAML, client certs, biometrics, MFA |
| Tampering | Integrity | HMAC, digital signatures, checksums, immutable logs |
| Repudiation | Non-repudiation | Signed audit logs, write-ahead logs, cryptographic receipts |
| Information Disclosure | Confidentiality | TLS, encryption at rest, column-level encryption, access control |
| Denial of Service | Availability | Rate limiting, auto-scaling, CDN, DDoS protection, circuit breakers |
| Elevation of Privilege | Authorization | RBAC, ABAC, ACLs, principle of least privilege, privilege separation |

The threat model should produce both: a list of threats *and* a corresponding set of security requirements that, if met, neutralize or mitigate those threats.

## Build It

You will complete four steps in `code/notes.md`. Each step builds on the previous one and produces a section of the final threat model. Open `code/notes.md` and follow along — the reference sheet and blank worksheets are waiting there.

### Step 1: Create a DFD for a Note-Taking Web Application

Model a simple note-taking web app as a DFD using ASCII:

```
+----------+     HTTPS      +------------+     SQL      +----------+
|  Browser |───────────────>| Web Server |────────────>| Database |
|  (User)  |<───────────────|  (Process) |<────────────| (Data    |
+----------+                +------------+             |  Store)  |
                               |    ^                   +----------+
                               |    |
                               |    | SQL
                               v    |
                            +-----------+
                            | Session   |
                            | Store     |
                            | (Data     |
                            |  Store)   |
                            +-----------+

Trust boundaries:
[Internet] --- dashed line --- [Server-side]
```

Elements identified:
- **External Entity:** User (browser) — outside our control.
- **Process:** Web server — handles HTTP requests, session management, and database queries.
- **Data Store:** Database — stores notes, user accounts, credentials (hashed).
- **Data Store:** Session store — stores active session tokens.
- **Data Flow:** HTTPS — data flow from browser to web server (crosses trust boundary).
- **Data Flow:** SQL — data flow from web server to database (within trust boundary).

### Step 2: Apply STRIDE Per Element

For each element, list specific threats (document in `code/notes.md` under the STRIDE worksheet):

| Element | Threat Type | Specific Threat Description |
|---------|-------------|---------------------------|
| **User (External Entity)** | Spoofing | Attacker impersonates a legitimate user (credential stuffing, session hijacking, phishing). |
| **Web Server (Process)** | Spoofing | Attacker tricks the server into thinking a request came from an admin (IP spoofing, header manipulation). |
| **Web Server (Process)** | Tampering | Attacker modifies request data in transit (if HTTPS is stripped or misconfigured, plaintext injection). |
| **Web Server (Process)** | Repudiation | Malicious user performs an action and denies it (no audit trail). |
| **Web Server (Process)** | Info Disclosure | Server leaks stack traces, debug info, or internal IPs via error messages or verbose headers. |
| **Web Server (Process)** | DoS | Attacker floods the server with requests, exhausting connection pool or CPU. |
| **Web Server (Process)** | EoP | Attacker accesses admin endpoints without authorization (broken access control, missing role check). |
| **Database (Data Store)** | Tampering | Attacker modifies stored notes or user data via SQL injection. |
| **Database (Data Store)** | Info Disclosure | Database credentials leaked; unencrypted PII exposed at rest. |
| **Database (Data Store)** | DoS | Attacker fills disk space via blob insertion or runs expensive queries to starve connections. |
| **Session Store (Data Store)** | Tampering | Session fixation: attacker forces a known session ID onto a victim. |
| **Session Store (Data Store)** | Info Disclosure | Session token leak via log files, referrer headers, or shared hosting. |
| **HTTPS (Data Flow)** | Spoofing | Attacker performs a man-in-the-middle attack with a rogue certificate (if CA validation is weak). |
| **HTTPS (Data Flow)** | Tampering | Downgrade attack: force HTTP, modify response content. |
| **SQL (Data Flow)** | Spoofing | Attacker tricks the web server into connecting to a rogue database (DNS hijacking). |
| **SQL (Data Flow)** | Info Disclosure | Query results returned to unauthorized user (IDOR — insecure direct object reference). |

### Step 3: Build Attack Trees for High-Priority Threats

From the STRIDE analysis, pick the most critical threats and build attack trees. Here is the tree for **"Read another user's notes":**

```
GOAL: Read another user's notes
├── OR: SQL injection on notes endpoint
│   ├── AND: Find injectable parameter (notes GET parameter 'id')
│   ├── AND: Bypass input filter (WAF, parameterized query check)
│   └── AND: Craft UNION SELECT to extract other users' note content
├── OR: Session hijacking
│   ├── AND: Steal session cookie
│   │   ├── OR: Cross-Site Scripting (XSS) — inject script that reads document.cookie
│   │   │   ├── AND: Find stored XSS in note content
│   │   │   └── AND: Victim visits note page with injected script
│   │   ├── OR: Intercept unencrypted connection (FAIL — HTTPS mandatory)
│   │   └── OR: Read cookie from access log (FAIL — HttpOnly flag set)
│   └── AND: Use stolen cookie to impersonate victim session
├── OR: Insecure Direct Object Reference (IDOR)
│   └── AND: Guess or enumerate note IDs in /notes/{id}
│       └── AND: Endpoint returns note without ownership check
├── OR: SSRF to access database directly
│   ├── AND: Find server-side endpoint that makes requests to user-supplied URLs
│   └── AND: Point SSRF to internal database API or network share
└── OR: Database backup leak
    ├── AND: Backup file exposed on public web server (.sql, .dump)
    └── AND: Read backup containing plaintext note data
```

AND/OR semantics:
- **OR** at each branch means the attacker can pick the easiest available path.
- **AND** means all sub-goals must succeed — a single failure blocks that path.
- FAIL nodes are pruned: the path is infeasible (e.g., "Intercept unencrypted connection" fails because HTTPS is enforced).

### Step 4: Risk Assessment with DREAD

Score each feasible attack path using DREAD (document in `code/notes.md` DREAD worksheet):

| Threat Path | D | R | E | A | D | Total | Priority |
|-------------|---|---|---|---|---|-------|----------|
| SQL injection | 3 | 2 | 2 | 1 | 3 | 11/15 | HIGH |
| XSS → cookie theft → session hijack | 3 | 3 | 2 | 1 | 2 | 11/15 | HIGH |
| IDOR — guess note IDs | 3 | 2 | 2 | 1 | 2 | 10/15 | HIGH |
| SSRF → internal database | 3 | 1 | 1 | 1 | 1 | 7/15 | MEDIUM |
| Database backup leak | 3 | 1 | 1 | 3 | 1 | 9/15 | MEDIUM |

**Interpretation:**

- **SQL injection (11/15) and XSS session hijack (11/15)** are the top two threats. Both score high in Damage (full note data exposure), Reproducibility (reliable exploit once the vulnerability exists), and Discoverability (well-known vulnerability classes with automated tools).
- **IDOR (10/15)** is also high — discovery is easy (just enumerate IDs in the URL) and damage is total, but exploitability depends on the app not checking ownership.
- **SSRF (7/15)** and **backup leak (9/15)** are lower priority — they require more specific conditions (SSRF gadget endpoint exists, backup is accidentally public). They are still worth fixing but can be deferred behind the high-priority items.

**Action plan based on DREAD:**
1. **Immediate (current sprint):** Fix SQL injection (parameterized queries), fix XSS (output encoding + CSP), fix IDOR (ownership checks on all note endpoints).
2. **Next sprint:** Fix backup exposure (restrict public access to backup directory, encrypt backups).
3. **Backlog:** Investigate SSRF risk (audit all HTTP client usage, validate URLs).

This is the output of a complete threat model: a prioritized list of actionable fixes that an engineering team can execute.

## Use It

Threat modeling appears at multiple scales across the industry:

- **Microsoft SDL (Security Development Lifecycle):** Microsoft formalized STRIDE as part of its SDL in the early 2000s. Every feature at Microsoft goes through a threat modeling review at design time, using the Microsoft Threat Modeling Tool (free, GUI-based, generates STRIDE-per-element reports). The SDL includes mandatory training, tooling integration in Visual Studio, and annual security audits.

- **Google's BeyondCorp:** Google's zero-trust architecture (BeyondCorp) was designed using threat modeling. The core insight: trust the network, not the device. A threat model of the traditional VPN-based approach revealed that a compromised device inside the network could access everything — the threat model drove the architectural shift to per-resource access decisions based on device posture and user identity.

- **AWS Threat Composer:** AWS provides a free, web-based threat modeling tool (docs.aws.amazon.com/threat-composer) that combines STRIDE, attack library, and automated report generation. It includes a knowledge base of common cloud threats organized by AWS service. Teams can model their architecture using AWS service icons, apply STRIDE, and export findings.

- **OWASP Threat Dragon:** An open-source threat modeling tool (threatdragon.org) with a similar workflow: draw DFDs, apply STRIDE, generate reports. It runs as a desktop app or a web app and outputs findings in JSON format that can integrate with issue trackers.

- **C4 Model:** Simon Brown's C4 model (Context, Container, Component, Code) is used by many teams to visualize software architecture before applying threat modeling. The C4 model's container and component diagrams map naturally to DFDs — containers become processes, databases become data stores. The structured decomposition of C4 makes it easier to apply STRIDE at the right level of granularity.

**Comparison to what you built:** Your ASCII DFD, STRIDE worksheet, and DREAD scoring in `code/notes.md` cover the same workflow as the commercial tools — just without the GUI. The advantage of doing it manually is that the *thinking* cannot be automated. A tool can prompt you with "what if an attacker spoofs this external entity?" but it cannot know your architecture's specific trust relationships or business context. The worksheet you built is the core artifact; the tools just make it prettier and more shareable.

## Read the Source

- **Microsoft Threat Modeling Tool** — The canonical reference for STRIDE-based threat modeling. Download the tool and reverse-engineer a sample model to see how Microsoft expects STRIDE to map to DFD elements. Every diagram element has a STRIDE properties sheet.
- **"Threat Modeling: Designing for Security" by Adam Shostack** — Shostack was a principal program manager on Microsoft's SDL team. The book is the definitive practical guide. Read chapters on STRIDE per-element (ch. 5–7), DREAD (ch. 8), and attack trees (ch. 9). The book also covers the "Elevation of Privilege" card game, a threat-modeling exercise played with a STRIDE-themed deck.
- **"Attack Trees" by Bruce Schneier (Dr. Dobb's Journal, 1999)** — The original article that introduced attack trees. Schneier walks through examples for physical security (vault door) and computer security (PBX fraud). The article is short (~10 pages) and still the clearest explanation of the concept. Available at schneier.com.
- **OWASP Threat Modeling Cheat Sheet** — A concise, practical guide that covers STRIDE, attack trees, and the threat modeling process in a single page. Updated regularly with new attack patterns and references. Useful as a quick lookup during reviews.
- **C4 Model (c4model.com)** — Simon Brown's C4 model is not a threat modeling technique itself, but the structural decomposition it provides (context → containers → components → code) maps naturally to the granularity needed for STRIDE analysis. The C4 website includes examples of mapping C4 diagrams to threat models.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A threat modeling template** with DFD notation, STRIDE per-element worksheet, attack tree template (ASCII), DREAD scoring sheet, and trust-boundary checklist — ready to print or copy into any project's security review documentation.

## Exercises

1. **Easy** — Draw a DFD for a system you work with or know well (a food delivery app, a social media platform, a CI/CD pipeline). Identify at least two trust boundaries. List one STRIDE threat per element (6+ threats total).

2. **Medium** — Take the STRIDE threats from Exercise 1 and score each using DREAD. Produce a priority-ordered list with scores. Then pick the highest-priority threat and build an attack tree with at least four leaf nodes (use at least one AND node). Prune any branches that are infeasible given your system's controls (explain why).

3. **Hard** — Model a multi-tenant SaaS application with three trust boundaries: (1) Internet → load balancer, (2) load balancer → application tier, (3) application tier → database tier. Tenant data is stored in the same database with a `tenant_id` column. Apply STRIDE per element across all three boundaries. Build attack trees for: "exfiltrate another tenant's data" and "elevate from regular user to tenant admin." Score all paths with DREAD. Produce a prioritized mitigation plan with specific controls for each high-risk path.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| STRIDE | A threat categorization framework | Six categories (Spoofing, Tampering, Repudiation, Information Disclosure, DoS, Elevation of Privilege) mapping to six security properties. |
| DREAD | A risk scoring system | Five-dimension numeric score (Damage, Reproducibility, Exploitability, Affected Users, Discoverability), each rated 1–3, summed for priority ranking. |
| Attack tree | A hierarchical diagram of attack paths | A tree with the attacker's goal as root, OR/AND nodes as methods, and leaf nodes as concrete actions. |
| DFD | Data Flow Diagram | A diagram showing external entities, processes, data stores, and data flows, with trust boundaries drawn as dashed lines. |
| Trust boundary | A security perimeter | A dashed line across which data moves from one trust level to another — every crossing is a threat model opportunity. |
| Threat | A potential security violation | Something that *could* go wrong; the "what if" in threat modeling (before a vulnerability exists). |
| Vulnerability | A specific weakness in a system | A concrete bug or misconfiguration that an attacker can exploit (after the threat has materialized). |
| Risk | The combination of likelihood and impact | Threat × Vulnerability × Consequence; what DREAD attempts to quantify. |
| Mitigation | A control that reduces risk | A security measure (technical or procedural) that prevents, detects, or responds to a threat. |
| Security requirement | A system property that must hold | A statement of what "secure" means for this system (precedes mitigations). |
| Threat actor | Who might attack the system | Nation-state, organized crime, insider, script kiddie, hacktivist — each has different capabilities and motivation. |
| Per-element analysis | Applying STRIDE to each DFD element individually | The Standard practice: list every element, check which STRIDE categories apply, describe the threat. |

## Further Reading

- Adam Shostack, *Threat Modeling: Designing for Security* (Wiley, 2014). The definitive book on the subject. Covers STRIDE, DREAD, attack trees, the SDL threat modeling process, and the Elevation of Privilege game.
- Bruce Schneier, "Attack Trees," *Dr. Dobb's Journal*, December 1999. The original paper that introduced attack trees. Still the best brief introduction to the concept.
- Microsoft, "Threat Modeling Security Fundamentals" (docs.microsoft.com/en-us/azure/security/develop/threat-modeling-tool). Microsoft's official documentation for STRIDE-based threat modeling, including the Threat Modeling Tool download and walkthrough guides.
- OWASP, "Threat Modeling Cheat Sheet" (cheatsheetseries.owasp.org). A single-page reference for STRIDE, attack trees, and the threat modeling workflow, maintained by the OWASP community.
- National Institute of Standards and Technology (NIST), "Guide to Data-Centric System Threat Modeling" (NIST SP 800-154, draft). A data-centric approach to threat modeling that focuses on where data flows, where it is stored, and who can access it — complementary to STRIDE's element-centric approach.
