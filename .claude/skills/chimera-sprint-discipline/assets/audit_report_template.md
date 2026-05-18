## Audit Report: {scope}

**Files read:**
- `{path_1}` ({n_lines} lines)
- `{path_2}` ({n_lines} lines)

**Findings:**

| Q# | Question | Answer | Evidence | Risk |
|---|---|---|---|---|
| Q1 | {audit_question} | {one_sentence_answer} | `{file}:{line}` | Low/Med/High |
| Q2 | ... | ... | ... | ... |

**Cross-references discovered:**
- {symbol_X} used in {n} sites: `{file_a}:{line}`, `{file_b}:{line}`, ...

**Audit complete.** {n} questions answered, {m} file:line references, {k} cross-references.

**Suggested next:** [`sprint_planning` for {scope}] OR [discussion of {specific_concern}]
```

---

## `chimera-sprint-discipline/assets/sprint-prompt-template.md`

```markdown
## Sprint {phase}.{number}: {one_phrase_title}

**Friction reference:** `{friction_id}` ({status})
  OR
**Anticipatory justification:** {one_paragraph_reason}

### 目标
{single_sentence_objective}

### 设计要点
- {architectural_decision_1}
- {architectural_decision_2}

### 任务范围
1. {task_1} (`{file_path}`, ~{n} lines)
2. {task_2} (`{file_path}`, ~{n} lines)
3. {task_3} (`{file_path}`, ~{n} lines)

### 验收
- {measurable_criterion_1}
- {measurable_criterion_2}

### 红线
- ❌ 不修改 `{forbidden_file_or_module}`
- ❌ 不引入 `{forbidden_dependency}`
- ❌ 不破坏 {forbidden_behavior}
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `{primary_path}` 等
- 测试: `tests/{test_path}`
- 文档: `docs/ARCHITECTURE/{doc_name}.md` (sprint末尾更新)

### 下游消费者
This sprint's output will be consumed by `chimera-code-taste` skill in execution mode. The taste skill will run pytest/ruff/mypy after each modification. Ensure 验收 criteria are pytest-verifiable where possible.