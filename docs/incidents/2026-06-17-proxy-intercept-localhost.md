Root cause: System proxy (127.0.0.1:7890) intercepts Astrocyte
  → Oligo localhost:33333 connection. reqwest reads HTTP_PROXY env var.
Fix: (1) Proxy config bypass localhost + (2) Astrocyte launch script
  clears HTTP_PROXY.
Prevention: Add to CLAUDE.md under "Environment" — note that system
  proxy must bypass 127.0.0.1 for local services.