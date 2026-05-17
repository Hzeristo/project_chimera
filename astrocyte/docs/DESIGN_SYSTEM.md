# Astrocyte 设计系统

Sprint 1 在 `src/app.css` 的 `:root` 中落地了设计 Token；本文说明**语义与用法**，避免第二 Accent 滥用。

---

## 色彩系统

### 主色调：Neural Purple

- **用途**：所有交互元素的默认状态（按钮、边框、高亮、Skill/Persona HUD、聊天气泡强调等）。
- **核心变量**：`--astrocyte-neural-purple`（`#bb9af7`）
- **透明度阶梯**：`--astrocyte-purple-a-04` … `--astrocyte-purple-a-95`（与 `rgba(187, 154, 247, α)` 对齐）
- **便捷别名**：`--astrocyte-purple-border`、`--astrocyte-purple-subtle`、`--astrocyte-purple-glow` 等（见 `app.css`）

### 第二 Accent：Cyan–Violet 渐变

- **用途**：**仅用于进度指示器与长时间运行任务的可视化**（例如 Miner 任务进度条填充）。
- **变量**：`--astrocyte-accent-cyan`（`#22d3ee`）、`--astrocyte-accent-violet`（`#7c3aed`）
- **禁止用于**：普通按钮、卡片悬停边框、正文/标签文字高亮、Skill Chip 等与「任务进度」无关的 UI。

### 语义色

- **Good / Success**：`--feedback-good`（`#8ef1b6`）
- **Bad / Error（用户反馈态等）**：`--feedback-bad`（`#ff8fa3`）
- **Warning**：`--warning`（`#f59e0b`）
- **流式错误条等**：见 `app.css` 中 `--error`、`--error-surface`、`--error-fg`

### 系统遥测 / 日志蓝（非第二 Accent）

- 系统日志、遥测前缀等使用的青蓝系为**独立语义**，**不要**与 Cyan–Violet 进度条混为一谈；沿用 `app.css` 中 `.system-log-raw` 等既有变量与色值。

### 背景、遮罩与语义 Surface（字面 `rgba` / `#` 收拢）

基座级 surface（`--surface-0` … `--surface-body`、`--surface-modal*`、`--surface-chrome-*` 等）见 `app.css` 中 `:root`「背景层级」注释块。以下补充**遮罩、嵌套暗区、侧栏/浮层、状态点、进度槽**等专用变量（与实现一一对应）：

| 变量 | 典型用途 |
|------|----------|
| `--surface-scrim` | 全屏模态遮罩（如设置层） |
| `--surface-embed` | 嵌套列表/左栏等偏暗底（如军械库列表） |
| `--surface-embed-deep` | 更暗一层的嵌套区（如军械库右侧编辑区） |
| `--surface-input-dim` | 设置内表单输入区底色 |
| `--surface-sidebar` | 侧栏 Webview 毛玻璃壳 |
| `--surface-floating` | 时间轴等固定浮层提示卡片 |
| `--surface-tile-90` | 列表项/_tile 行（如军械库条目行） |
| `--surface-card-frost` | 侧栏 Memory 卡片类毛玻璃底 |
| `--surface-scrollbar-track` | 全局滚动条轨道；与 `scrollbar-color` 中拇指色成对使用 |
| `--surface-bright` | 高亮白点（时间轴节点默认态、Provider LED「在线」等） |
| `--surface-progress-track` | 矿工任务进度条**槽**（条身渐变仍用第二 Accent，见上表） |
| `--surface-danger-surface` | 危险操作轻衬（如时间轴删除按钮悬停） |
| `--status-led-mute` | Provider 连接 LED 默认灰 |
| `--status-led-mute-dim` | `data-state="idle"` 时略压暗 |
| `--status-led-probe` | 探测/握手态（`probing`） |
| `--status-led-off` | 离线/不可用（`down`） |

**说明**：装饰性 `linear-gradient`（时间轴能量线、遥测条叠层等）**不**再拆成独立 token，色停可继续在 `gradient` 内写 `rgba` 或现有紫色阶梯变量，避免 token 数量膨胀。

---

## 使用示例

### 正确

```css
.miner-task-bar-fill {
  background: linear-gradient(90deg, var(--astrocyte-accent-violet), var(--astrocyte-accent-cyan));
}
```

### 错误

```css
.skill-card:hover {
  border-color: var(--astrocyte-accent-cyan); /* 应使用主色，如 var(--astrocyte-purple-border) 或 var(--astrocyte-neural-purple) */
}
```

```css
.skill-active-chip {
  border-color: rgba(139, 92, 246, 0.4); /* 应使用主色 Token，如 var(--astrocyte-purple-border) */
}
```

---

## 与代码库的对照

- **全局 Token 定义**：`src/app.css` 文件顶部 `:root`（含上表 `--surface-*`、`--status-led-*` 等）
- **本文**：色彩语义、第二 Accent 边界、Surface/遮罩补充与 `linear-gradient` 约定
- **历史审计（风格一致性）**：仓库根目录 `docs/Astrocyte_UI_Style_Consistency_Report.md`
