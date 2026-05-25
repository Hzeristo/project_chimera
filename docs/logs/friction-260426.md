## 使用摩擦报告

### entry 1
- 想做的事：使用astrocyte深读一篇名字叫Hermes的paper，paper的idea是MLLM的视频流推理中用hierarchical kvcache做成agent memory。
- 实际怎么做的：手动下载pdf，手动转换成md，手动阅读，切换到终端跑 run_lens。
- 为什么没用astrocyte：前端无法操作文件。
- 成本：20分钟的认知债务
- 理想：一句话，让astrocyte读取本地paper存储中下载下来的文献，根据skill，自主识别意图，并进行深度阅读，落盘vault形成node的同时，在前端能看到BB的嘲讽。

### entry 2
- 想做的事：使用astrocyte访问obsidian vault
- 实际怎么做的：发现工具调用很成功，但是没能fetch到我需要的文件。同时反馈ui很丑陋，e/d/r按钮经过之前的token化之后也很丑陋
- 为什么没用astrocyte：用了。
- 成本：1分钟手动寻找。
- 理想：解决ui丑陋问题之后，优化vault搜索逻辑，能除了关键词匹配，正经语义查询查出来。

### entry 3
- 想做的事：使用astrocyte访问精读后的报告
- 实际怎么做的：手动查找。
- 为什么没用astrocyte：用了，但是你看日志：

```
Rules:
1. If a tool is needed, output <CMD:tool_name({JSON object})> with a single JSON object as the argument (e.g. {"query": "..."}). You may output more than one <CMD> if required.
2. If no tool is needed (e.g. greeting, small talk, or no external data is required),
2026-04-26 14:56:05 | INFO     | httpx | HTTP Request: POST https://api.deepseek.com/chat/completions "HTTP/1.1 200 OK"
2026-04-26 14:56:29 | INFO     | src.oligo.core.agent | [Router] Full response (probe): 从 vault 中已定位到笔记 ：

**文件：** `2410.10813v2-LONGMEMEVAL_Deep_Read.md`

**元数据：**
- `type`: knowledge
- `chimera_status`: deep_read
- `arxiv_id`: "2410.10813v2"
- `short_moniker`: "LONGMEMEVAL"
- `tags`: [deep_read, "📄/optics"]
- `processed`: "2026-04-14"
- `source_md`: "D:/MAS/crucible_core/papers/filtered/Skim/2410.10813v2-LONGMEMEVAL.md"
- `architecture_types`: ["Retrieval-Augmented Generation (RAG)", "Modular Memory Pipeline", "Online Context Compression", "Prompt-Bas..."] *(内容截断)*

该笔记还被另一条笔记 `Thought-memory bench-implementation pitfalls.md` 通过 wikilink 引用。

如需获取正文全部内容，建议使用 `obsidian_graph_query` 或直接打开源 Markdown 文件 (`source_md` 路径)。是否需要我 进一步拉取完整正文？
2026-04-26 14:56:29 | INFO     | src.oligo.core.agent | [Router] probe_end tool_calls=0
2026-04-26 14:56:29 | INFO     | src.oligo.core.agent | [Final] begin (persona bind + generate buffer)
2026-04-26 14:56:29 | INFO     | src.oligo.core.agent | [Final] FINAL PERSONA SYS (first 150 chars): [SYSTEM: REVIEWER ZERO — PRE-FLIGHT AUDIT STUB]
You are Reviewer Zero.
Primary objective: identify defects, risks, edge cases, regressions, and missin
2026-04-26 14:56:30 | INFO     | httpx | HTTP Request: POST https://api.deepseek.com/chat/completions "HTTP/1.1 200 OK"
2026-04-26 14:56:43 | INFO     | src.oligo.core.agent | [Final] Full response (final stream): <CMD:read_vault_file("2410.10813v2-LONGMEMEVAL_Deep_Read.md")>
2026-04-26 14:56:43 | INFO     | src.oligo.core.agent | [Final] buffer_ready chars=62 sse_chunking

```

找到了之后硬是返回cmd指令

- 成本：一杯咖啡。气的。
- 理想：不要发生这种很无聊的错误。

### entry 4
- 想做的事：长尾任务调用。
- 痛点：前端没有计时器**实时**标定**各个步骤**的**精确**耗时。

### entry 5
- 想做的事：使用astrocyte开始早上paper fetch和筛选
- 实际怎么做的：使用 run_daily 脚本
- 为什么没用astrocyte：没连上。
- 成本：5分钟。半杯咖啡
- 理想：能从astrocyte开始调用所有的miner工具和脚本。

### entry 6 [Status: RESOLVED — Phase III.C]
- 时间: 2026-05-25
- 想做: 追溯 BB 回答中引用的 vault 文件（E3 friction resolution）
- 实际: Phase III.C FC.1–FC.3b 实现了 ToolOutput/Artifact 结构化输出、bb-message-artifacts SSE 事件、Svelte artifact chip 渲染、open_vault_note Tauri 命令。点击 chip 直接在 Obsidian 打开对应笔记。
- 解决: FC.1 (`4a4cf0c`) + FC.2a (`094f28d`) + FC.2b (`dcb9807`) + FC.3a (`4261a84`) + FC.3b (`b7582bf`)
- 参考: `docs/ARCHITECTURE/FINAL_CONTRACT.md` §1–3, §5–6

### entry 7 [Status: RESOLVED — Phase III.C]
- 时间: 2026-05-25
- 想做: 删除 BB 消息（E4 friction resolution）
- 实际: Phase III.C FC.5 verify-only sprint 确认 delete pipeline 完整：delete_chat_message Tauri 命令、memory.rs delete_entry、前端 deleteMessage + onAiAction 全部到位，删除后重启持久化。
- 解决: delete pipeline 已在 Phase III.B 期间实现；FC.5 (`d75dec1`) 正式核查并记录
- 参考: `docs/audits/FC.5-verify.md`, `docs/ARCHITECTURE/FINAL_CONTRACT.md` §7

