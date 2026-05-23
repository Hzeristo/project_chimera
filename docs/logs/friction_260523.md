### Entry 1 [Status: OPEN]
- 时间: 2026.05.23
- 想做: 写 FC.2a 的 smoke test
- 实际: 卡在 mock 不切 PASS 导致 max_turns 耗尽
- 根因:
  (a) conftest.MockLLMClient 没有 PASS-switch 逻辑
  (b) agent.py __main__ 有该逻辑但是 test_through_main 反模式
  (c) 现有 test_run_theater_with_tool_calls_executes_and_streams 可能掩盖了 mock 缺陷
- 成本: FC.2a 进度受阻
- 理想: 统一 mock harness;debt week 移除 __main__ 测试代码
