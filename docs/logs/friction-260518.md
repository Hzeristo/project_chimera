### Entry 1 [Status: CLOSED]
- 时间: 2026.05.18
- 想做: 启动 Phase III.C 第一个 sprint
- 实际: 发现 ports/papers/ 整个目录在 migration 中消失,卡住
- 根因:
  (a) migration subtree merge 没把所有文件带过来
  (b) skill 体系没有 recovery mode,code-taste modification 不适合处理这种级联故障
  (c) 没有"完整性 check"机制(commit hook / CI)在 migration 后扫描 broken imports
- 成本: Phase III.C 推迟 1+ 小时
- 理想: chimera-recovery skill;migration 后自动 import-resolution scan

### Entry 2 [Status: SCHEDULED]
- 时间: 2026.05.18
- 想做: 在干净环境下跑 R.1 验证
- 实际: 仍在用 PaperMiner 时代遗留的 conda 环境,臃肿且混杂
- 根因:
  (a) 项目从未明确声明开发环境(venv/conda/uv?Python 版本?)
  (b) skill 体系没有强制环境约束,Claude 可能在错误的 Python 上跑工具
  (c) ruff/mypy 之前根本没在这个项目跑过——R.1 是第一次发现
- 成本: 隐性技术债 + 跨项目工具污染风险
- 理想: skill 明确声明环境路径 + 工具入口;debt week 清理 conda env

