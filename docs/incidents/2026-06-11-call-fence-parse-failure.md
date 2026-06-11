Root cause: router_intro.md.j2 examples wrap <tool_call> in ```xml fences;
  DeepSeek mimics the fence; parse_tool_calls strips code blocks (S0.4
  legacy) → real tool calls dropped → tool_calls=0, backfilled as prose.
Fix: (A) parser matches <tool_call> by XML tag regardless of fence;
     (B) router_intro.md.j2 examples use bare XML, no ```xml fence.
Regression test: smoke_router_fenced_toolcall.py