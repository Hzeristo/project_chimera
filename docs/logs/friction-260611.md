### Entry 1 [Status: RESOLVED — Phase IV.A, commit e561581/4109c78]
- 时间：2026.06.13
- 想做：用chimera爬取paper
- 实际：返回：

    ```
    2026-06-13 11:03:31 | INFO     | src.oligo.core.agent | [Router] Full response (probe): 论文爬取管道已启动！

    任务已提交，正在后台运行。任务 ID 为 **`500b7ad6`**。你可以稍后使用以下命令查看进度：

    ```
    check_task_status('500b7ad6')
    ```

    现在任务刚启动，暂时还没有结果，等跑一会儿再查即可。
    2026-06-13 11:03:31 | INFO     | src.oligo.core.agent | [Router] probe_end tool_calls=0
    2026-06-13 11:03:31 | INFO     | src.oligo.core.agent | [Router] probe_draft_backfill chars=127 (raw_len=131)
    2026-06-13 11:03:31 | INFO     | src.oligo.core.agent | [Final] begin (persona bind + generate buffer)
    2026-06-13 11:03:31 | INFO     | src.oligo.core.agent | [Final] FINAL PERSONA SYS (first 150 chars): [SYSTEM: REVIEWER ZERO — PRE-FLIGHT AUDIT STUB]
    You are Reviewer Zero.
    Primary objective: identify defects, risks, edge cases, regressions, and missin
    2026-06-13 11:03:31 | INFO     | httpx | HTTP Request: POST https://api.deepseek.com/chat/completions "HTTP/1.1 200 OK"
    2026-06-13 11:03:32 | INFO     | src.crucible.ports.arxiv.arxiv_fetch | [Arxiv] Fetched 100 arXiv records since 2026-06-10
    2026-06-13 11:03:43 | INFO     | src.crucible.ports.arxiv.arxiv_fetch | [Arxiv] Downloaded PDF: 2606.13680.pdf
    2026-06-13 11:03:46 | INFO     | src.crucible.ports.arxiv.arxiv_fetch | [Arxiv] Downloaded PDF: 2606.13673.pdf
    2026-06-13 11:03:55 | INFO     | src.crucible.ports.arxiv.arxiv_fetch | [Arxiv] Downloaded PDF: 2606.13669.pdf
    2026-06-13 11:03:56 | INFO     | src.oligo.core.agent | [Final] Full response (final stream): 以下是对 `daily_paper_pipeline` 交互的审计结论，聚焦风险、缺失测试与潜在回归。

    ### 🔴 高风险项
    1. **无输入参数，行为隐式依赖**  
    - 没有指定日期范围、论文源（arXiv/SemanticScholar）、主题/关键词，完全依赖系统默认值。  
    - 风险：默认值变更会导致相同调用产生不同结果，难以复现与排错；多用户下若默认值基于全局状态，可能互相干扰。

    2. **异步任务无完成反馈机制**  
    - 只返回任务 ID，用户需手动轮询 `check_task_status`，没有 callback/webhook/消息推送。  
    - 风险：若任务失败或长时间挂起，用户无法及时获知；长时间轮询可能浪费资源。

    3. **无可观测性指标**  
    - 未记录管道启动时间、数据源健康状态、预期耗时等。  
    - 风险：运维无法快速区分“还在跑”和“已经死了”，导致错误判断。

    4. **缺少幂等性与去重**  
    - 若同一用户重复调用，可能启动多个相同任务（如两个管道同时爬取同一天），产生重复论文数据。  
    - 风险：数据重复污染下游应用，且无保护措施。

    ### 🟡 中风险项
    5. **任务 ID 的权限模型不明**  
    - `check_task_status('500b7ad6')` 是否允许任何用户查询？若未做隔离，可能泄漏其他用户任务状态。  
    6. **无超时与取消机制**  
    - 管道可能无限期运行，缺少 `cancel_task` 或 `timeout` 参数。  
    7. **错误处理不透明**  
    - 启动成功仅表示任务被接受，无法反映后续爬取源的网络异常、解析失败、存储故障等。

    ### 🔍 缺失的测试用例（关键测试计划）
    | 类别 | 测试场景 | 预期结果 |
    |------|----------|----------|
    | **基础功能** | 调用 `daily_paper_pipeline`，轮询状态至 COMPLETED | 返回论文列表，格式符合约定，数量合理 |
    | **幂等性** | 短时间内连续调用 3 次 | 仅创建 1 个有效任务，其余返回“已存在”或取消 |
    | **重复启动** | 任务运行中再次调用 | 返回已有任务 ID 或拒绝（取决于策略） |
    | **权限隔离** | 用户 A 创建任务，用户 B 调用 `check_task_status` 用 A 的 ID | 被拒绝或无权限 |
    | **超时处理** | 模拟爬取源无响应，管道运行超过 30 分钟 | 自动标记 FAILED 并返回部分结果/错误日志 |
    | **资源限制** | 同时启动 100 个管道任务 | 排队或限流，系统不崩溃 |
    | **回归验证** | 更改默认数据源后，对比旧版本返回的论文数量/格式 | 确保向后兼容或版本化接口 |
    | **部分失败** | 一个数据源成功、另一个失败 | 任务状态标记为 PARTIAL，返回成功部分以及错误详情 |

    ### ❓ 待澄清需求（若缺失则会引入严重风险）
    - **数据源列表与优先级**：爬取 arXiv、会议网站，还是其他？若某源不可用，是否需要降级？
    - **日期指定方式**：是“今天”的论文，还是可传参？时区如何处理？
    - **输出格式与存储**：结果写入数据库、文件还是内存？是否有大小限制（如论文摘要最大长度）？
    - **重试策略**：网络瞬时故障是否需要内建重试？重试次数/间隔？

    这些问题若不明确，后续的回归测试将没有真正的通过标准。

    ### 🧪 建议立即补充的监控项
    - 任务生命周期事件日志（CREATED → RUNNING → SUCCESS/FAILED）
    - 每个数据源的调用耗时与成功率
    - 管道整体运行时长分位数（P50/P90）

    **结论**：当前实现仅是一个最简化的异步触发器，存在大量隐式契约与不可观测点。在进入生产或扩展到多用户前，必须补齐上述控制逻辑与测试。
    2026-06-13 11:03:56 | INFO     | src.oligo.core.agent | [Final] buffer_ready chars=1710 sse_chunking
    ```

    实际上后台进程进行过程中，最后的人格注入模块直接把“pending”变成幻觉文献输出；
 
- 根因：实际上不是人格注入方法过于简单，而是完全没有异步的工作组织。后端返回了异步工具调用，然后前端等都不等，直接返回“哎呀这样做不行”；
- 成本：需要手动去文件夹里面检查今日papers
- 理想：需要明确忙等待，监听，同时保证长周期任务结束之后oligo还能继续工作。

### Entry 2 [Status: RESOLVED]

- 时间：2026.06.11
- 想做：同样是使用chimera爬取papers
- 实际：路径配置错误，下载到了chimera所在根目录的单独crucible_core文件夹下。
- 根因：python工程bug，很有可能是cwd陷阱，
- 成本：手动init python script 
- 理想：正确配置python里面crucible相关代码逻辑；

### Entry 3 [Status: OPEN]

- 时间：2026.06.11
- 想做：创建新session相关时候的事情；
- 实际：创建新session的时候左侧timeline没有实时更新新小圆点，删除当前session的时候聊天框也不会刷新；
- 根因：astrocyte没这个逻辑；
- 成本：一罐330ml可乐。
- 理想：真实的根据jsonl状态的真实历史反应；