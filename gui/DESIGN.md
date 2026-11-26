# data_guard GUI 设计说明

基于 NSH（Native Shell）的桌面应用。设计体系遵循 NSH Design/Coding Guides：

## 设计体系

- **主题**：shadcn/ui neutral 默认主题 + 自定义语义色（`theme.json`）
  - `good`：健康（绿）
  - `warn`：警告（琥珀）
  - 危险直接用主题的 `destructive`
  - 界面代码中不出现原始色值，一律通过 `theme()` 语义 token 取色
- **ui 层**（`src/ui.rs`）：应用自己的样式组件层
  - `primary_button`：主操作按钮（基于 `nsh_base::button` 的行为：焦点、键盘、无障碍）
  - `badge`：健康等级徽章（色块透明度 12% + 同色文字）
  - `stat_bar`：比率条（4px 轨道 + 健康色填充）
  - 取色/圆角/尺寸只出现在 ui 层，页面（`lib.rs` / `report_view.rs`）只组合
- **排版**：标题 20/16px semibold，正文 14/13px，辅助信息 12px muted
- **目标平台**：桌面（Linux/GTK4 为主，兼容 macOS/Windows）；窗口 900×640

## 动效清单（Motion Inventory）

| 状态变化 | 工具 | 时长/缓动 |
|---|---|---|
| 按钮按下 | `.active(\|s\| s.opacity(0.85))` 按压水洗 | 即时，松开恢复 |
| 徽章/比率条颜色变化（主题切换、数据变化） | `.transition()`（颜色/透明度 150ms ease-in-out） | 150ms |
| 空态 → 扫描中 → 报告 | 内容整体替换（`dynamic`） | 无过渡（待办：淡入） |
| 扫描中指示 | 文本标签（待办：indeterminate `Progress` 部件样式） | — |
| 列表滚动 | `virtual_list` 原生滚动 | 平台行为 |

遵循系统"减少动态"设置（NSH 平台层处理）。

## 键盘与无障碍

- 所有可交互元素基于 `nsh_base::button`：Tab 焦点、Enter/Space 触发、
  屏幕阅读器角色与名称（按钮文字）由基础组件提供
- 比率/评级同时以文字呈现（不只靠颜色区分）

## 待办（视觉层面）

- [ ] 报告出现时 150ms 淡入（`transition` + opacity）
- [ ] 扫描中换用 `Progress` 组件（需要给 track/indicator 部件写 `Style`）
- [ ] 空态配一个 SVG 图标（经 `ui::Icon`，遵守"图标用 SVG 家族"规则）
