# Helix-MCP-Learner

**MCP Server → CI-144 Tool Definition Auto-Learner — Helix mines directly, no panning.**

[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/Status-P1--Complete-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/Tests-17%20passed-brightgreen.svg)]()

> MCP Servers are "structured gold mines" — someone has already structured the chaotic external world for us.
> Helix doesn't need OCR to pan for gold; MCP-Learner mines directly.
> **Strip MCP's skin, extract CI-144's bones.**

---

## Positioning

MCP-Learner is the "translator" of the Helix ecosystem — it translates MCP Server tool interfaces into CI-144 standard tool definitions, enabling [Helix-Tentacle](https://github.com/Jasonmilk/Helix-Tentacle) to execute them directly.

**It does not execute tools; it only learns and extracts.**

```
External World (GitHub/Slack/Notion/Filesystem/…)
    ↑ Native APIs (REST/GraphQL/SDK)
MCP Server (structured tool interfaces)
    ↑ MCP Protocol (JSON-RPC 2.0 over stdio/SSE)
MCP-Learner (this project)
    ├── Discovery (scan MCP Servers)
    ├── Learning (tools/list → tool manifest + schema)
    ├── Extraction (strip MCP wrapper, extract API patterns)
    ├── Rating (auto-assign PFP Risk-Level)
    └── Generation (Tentacle-loadable plugin Manifest)
    ↓ CI-144 Tool Definitions (Manifest)
Helix-Tentacle (Execution)
    ↓
Tuck (Security Decisions)
```

---

## Core Philosophy (phyt-DNA v1.0)

| Philosophy | Manifestation |
|---|---|
| **Extreme Decoupling** | MCP-Learner doesn't know Tentacle's internals; Tentacle doesn't know MCP exists. They communicate via Manifest. |
| **Extreme Reuse** | Reuses MCP protocol, CI-144 protocol family, Tentacle plugin system. |
| **On-Demand Loading** | Only activates when learning a new MCP Server; cached results are used directly after learning. |
| **Event-Driven** | Event-driven, re-learns when MCP Server changes, no polling. |
| **Physical Facts First** | PFP Risk-Level based on actual tool operation risk (delete=CRITICAL, read=LOW). |
| **Determinism First** | Same MCP Server + same learning logic → same CI-144 tool definitions. |

---

## Current Status

**P1 Minimum Viable Verification Complete** ✅ (2026-08-30)

| Task | Content | Status |
|---|---|---|
| T1 | MCP Client foundation (stdio + JSON-RPC 2.0 + tools/list + tools/call) | ✅ Done |
| T2 | CI-144 tool extraction layer (CIN7 + CAPABILITY-13 + PFP risk rating) | ✅ Done |
| T3 | Tentacle plugin Manifest generator | ✅ Done |
| T4 | End-to-end verification: mock-mcp-server → learn → generate → validate | ✅ Done |
| T5 | Performance comparison + determinism verification | ✅ Done |

**Tests**: 17 all green (14 unit + 3 integration)

### Performance Data

| Metric | Value | Description |
|---|---|---|
| Learning process (4 tools) | ~107ms | One-time cost |
| MCP direct call | ~239μs | read_file average latency |
| Manifest load | ~131μs | Single tool read+parse |
| Risk rating | ~1.3μs | Single rating |
| **CI-144 re-encapsulation overhead** | **<1%** | Manifest load vs MCP call |

---

## Quick Start

```bash
# Clone
git clone https://github.com/Jasonmilk/Helix-MCP-Learner.git
cd Helix-MCP-Learner

# Build
cargo build --release

# Learn from an MCP Server, generate Tentacle plugin Manifests
./target/release/mcp-learner learn \
  --command python3 \
  --args tests/mock_mcp_server.py \
  --output ./plugins \
  --name mock-filesystem

# List learned tools
./target/release/mcp-learner list --plugins-dir ./plugins
```

### Example Output

```
✅ Learning complete!
   Server: mock-filesystem
   Tools: 4
   Output: ./plugins

   Load in Tentacle:
   tentacle --transport stdio --plugins-dir ./plugins
```

---

## PFP Risk Rating Rules (Auto)

| Tool Name Pattern | Risk-Level | Examples |
|---|---|---|
| `read_*` / `list_*` / `get_*` / `search_*` | LOW | read_file, list_issues, get_user |
| `create_*` / `update_*` / `write_*` / `send_*` | MEDIUM | create_issue, update_file, send_message |
| `delete_*` / `remove_*` / `execute_*` / `run_*` | CRITICAL | delete_repo, remove_file, execute_command |
| `*_all` / `*_system` / `*_admin` / `*_root` | CATASTROPHIC | delete_all, system_update, admin_override |

**Security Constraint**: CRITICAL/CATASTROPHIC level tools require manual confirmation before activation after learning.

---

## Template-Based Reuse (How It Works)

MCP-Learner generates **parameterized** Manifests, not hardcoded tool calls. The `parameters_schema` is standard JSON Schema:

```json
{
  "type": "object",
  "properties": {
    "name": {"type": "string", "description": "Contact name to query"}
  },
  "required": ["name"]
}
```

This means:
- `name` is a **parameter slot**, not a hardcoded value
- Anaphase (orchestration layer) fills in any value at runtime (Coco, Jim, any name)
- Tentacle only validates parameter types, doesn't care about specific values

**Architecture Separation**:
- **Tentacle (Hand)**: Stateless executor, zero generalization capability
- **Anaphase (Brain/Orchestrator)**: Intent recognition + parameter filling
- **Mind (Memory)**: L1 strategy persistence for high-frequency operations
- **MCP-Learner (Translator)**: Extracts parameterized schemas from MCP tools

> The hand only grasps; the brain decides whether to grasp an apple or a pear this time.
> MCP-Learner's role is to distill the "grasping motion" into a standard process, allowing the brain to freely fill in targets.

---

## Project Structure

```
Helix-MCP-Learner/
├── src/
│   ├── lib.rs              # Library root, re-exports
│   ├── main.rs             # CLI entry point
│   ├── mcp/
│   │   └── mod.rs          # MCP Client (stdio + JSON-RPC 2.0)
│   ├── ci144/
│   │   └── mod.rs          # CI-144 extraction + PFP risk rating
│   └── manifest/
│       └── mod.rs          # Tentacle Manifest generator
├── tests/
│   ├── integration_test.rs # End-to-end tests
│   ├── perf_test.rs        # Performance comparison tests
│   └── mock_mcp_server.py  # Mock MCP Server for testing
├── docs/
│   ├── DNA.md              # Constitution: 6 immutable principles
│   ├── RNA.md              # Loading protocol: how AI reads this repo
│   ├── PLAN.md             # Navigation: current phase + next phase preview
│   ├── GROWTH.md           # Growth records: last 3 health snapshots
│   └── decisions/          # Architecture Decision Records (ADRs)
├── Cargo.toml
├── README.md               # English (this file)
└── README.zh-cn.md         # Chinese version
```

---

## Helix Ecosystem

| Project | Role | Status |
|---|---|---|
| [Helix-Mind](https://github.com/Jasonmilk/Helix-Mind) | Brain (memory/cognition) | ✅ Core complete |
| [Anaphase-Helix](https://github.com/Jasonmilk/Anaphase-Helix) | Torso (orchestration/execution) | ✅ Pending decision |
| [Helix-Tentacle](https://github.com/Jasonmilk/Helix-Tentacle) | Hand (tool execution) | ✅ Complete |
| [Tuck](https://github.com/Jasonmilk/Tuck) | Immune System (security gate) | ✅ Complete |
| [Cellrix](https://github.com/Jasonmilk/Cellrix) | Skin (UI/display) | ✅ Complete |
| [BIND-19](https://github.com/CommonIntents/BIND-19) | Nervous System (CI-144 protocol) | ✅ Complete |
| **Helix-MCP-Learner** | **Translator (MCP → CI-144)** | **✅ P1 Complete** |

---

## Governance

This project follows **phyt-DNA Methodology v1.0**.

| Document | Purpose |
|---|---|
| [docs/DNA.md](docs/DNA.md) | Constitution: 6 immutable principles |
| [docs/RNA.md](docs/RNA.md) | Loading protocol: how AI reads this repository |
| [docs/PLAN.md](docs/PLAN.md) | Navigation: current phase + next phase preview |
| [docs/GROWTH.md](docs/GROWTH.md) | Growth records: last 3 health snapshots |
| [docs/decisions/](docs/decisions/) | Architecture Decision Records (ADRs) |

---

## License

Apache 2.0. Managed by phyt-DNA Methodology v1.0.

---

*Helix-MCP-Learner. Strip MCP's skin, extract CI-144's bones.*
