---
name: chimera-dependency-veto
description: Dependency and framework veto list for Chimera. Activate when considering adding new dependencies, adopting SOTA frameworks, or when an external library is proposed. Prevents SaaS-thinking contamination and framework creep.
---

# Chimera Dependency Veto

Before proposing any new dependency or external framework integration, check this list. If the proposal matches a veto category, it requires explicit architect override.

## Permanently Vetoed

### Agent Frameworks
- **LangChain / LlamaIndex**: replaces Chimera's hand-crafted orchestration with opinionated abstractions. Whole point of Chimera is to not have these.
- **CrewAI / AutoGen**: multi-agent frameworks oriented for generality. Chimera does sequential multi-agent by hand.
- **Outlines / Guidance**: constrained decoding. Requires local model weights. Chimera runs on API providers only.

### Observability
- **OpenTelemetry / Langfuse / Sentry**: distributed tracing is for multi-service systems. Chimera is local-first, single-user. Structured logs + event SSE suffice.

### Data Infrastructure
- **Redis / Kafka / RabbitMQ**: message queues for distributed systems. Chimera uses `asyncio.Queue` and persistent JSON.
- **Vector databases (Pinecone, Weaviate, Qdrant)**: until Phase IV Exocortex, no semantic search is needed. And Phase IV will likely use `chromadb` (pure Python, no service).

### Security
- **OAuth libraries, sandbox runtimes**: single-user local app. Trust boundary is the OS user account.
- **SSRF protection libraries**: no untrusted input sources.

### JSON Parsing
- **demjson / json5 / dirtyjson**: non-strict JSON parsing. Argument repair in Chimera is done by rule-based fixups in stdlib `json`. If fixup fails, fall through to DENIED — do not silently parse malformed data.

## Conditionally Vetoed

### UI Libraries
- **shadcn / Material UI / Ant Design**: if proposed for a new component, ask if existing design tokens suffice first. Usually yes.
- **jQuery / lodash**: if proposed for browser code, modern JS has native equivalents.

### Testing
- **pytest plugins (beyond `pytest-asyncio`)**: unless solving a specific pain, stdlib + asyncio suffices.
- **Hypothesis / property-based testing**: valid for pure logic (parsers, FSM), overkill for everything else.

### LLM Provider SDKs
- Only `openai` Python SDK. Other providers via OpenAI-compatible base URL.
- **Never** add native `anthropic` / `google-generativeai` / `cohere` SDKs.

## The SOTA Paper Test

When a paper proposes a shiny new mechanism (MemGPT, Reflexion, Toolformer, etc.):

1. **Read it for architecture**, not code.
2. **Ask**: Does this paper solve a friction we've logged?
3. **If yes**: extract the core insight, implement in 100 lines using existing Chimera primitives. Do not port the paper's codebase.
4. **If no**: file the idea in `docs/plunder_list.md` for future reference. Do not implement speculatively.

## The Override Protocol

If a dependency seems genuinely needed:

1. Write one-paragraph justification answering:
   - Which friction it solves
   - Why existing Chimera primitives don't work
   - What is the migration exit strategy if this dependency dies
2. Time-box a proof-of-concept
3. If POC succeeds, commit with `feat(deps): add X — justified by friction Y`
4. If POC reveals alternatives, close without merging

Never add dependencies because "everyone uses it" or "it's industry standard".
