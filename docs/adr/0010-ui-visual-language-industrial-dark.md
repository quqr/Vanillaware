# ADR-0010：UI 视觉语言为「工业暗色 + 细线框」，导航为左侧图标栏

- **状态**：已取代（2026-10-22，ADR-0017）
- **影响范围**：`src/styles.css`、`src/components/*`、`src/views/*`

## 背景

ADR-0008 锁定了「库 / 预览 / 转换」三视图与「不得与 QuadToSpine2D 雷同」，
但把具体设计语言留到实现前定稿。重构前必须先选一套可贯穿三视图的视觉规则，
否则库视图是网格卡片、预览是画布、转换是 JSON 树，三处会各长一套样式。

## 决策

- **视觉语言：工业暗色 + 细线框**。深炭灰底（`--bg-0..3`）、**无圆角**、1px 细边框
  （`--line` / `--line-strong`）、单一强调色青绿（`--accent`）；数值、路径、flags 一律等宽字体。
  语义色只用三个：青绿=通过、琥珀=提醒、砖红=失败。
- **导航：左侧 56px 图标栏**。三视图切换不占用顶栏，预览画布拿到最大高度；
  栏顶放 `VW` 字标作为唯一装饰，不使用 Logo 图形或渐变。
- **布局规则**：每个视图顶部一条 `.bar` 承载「操作 + 当前对象」，中间是可滚动主区，
  右侧或底部是属性/时间轴；状态栏全局唯一，位于最底部，显示扫描统计与当前 tag。
- 旧的 daisyUI 语义类（`btn-*` / `badge-*` / `card bordered`）**不再使用**；
  `styles.css` 改为纯 CSS 的设计令牌 + 组件类，避免主题层再造一套视觉语言。

## 权衡

代价：放弃现成组件库的交互细节（焦点态、按压态等）需要自己补齐；
深色无圆角在长文本场景下视觉偏硬，靠字号与留白层级来补偿。

收益：三视图共用一套令牌，风格一致；等宽数字让时间码、flags、diff 值天然对齐；
与 QuadToSpine2D 的深色三页桌面形态拉开距离。

## 备选方案

- **明亮纸面 + 工程标注**：适合截图留档与技术图纸感，但暗房式动画预览不占优。
- **暗色数据可视化**：时间轴与曲线当主角，视觉冲击强，但长时间使用疲劳，且更像仪表盘而非工具。

---

## English summary

The visual language is **industrial dark with hairline outlines**: charcoal surfaces, no rounded corners,
1px borders, a single teal accent, and a monospace face for every number, path and flag. Navigation is a
56px icon rail on the left (the three views never eat the top bar, so the preview canvas gets full height).
Each view has one `.bar` for actions and the current object, a scrollable body, and a property/timeline side
or bottom region; the status bar is global. daisyUI semantic classes are no longer used — `styles.css` is a
plain token + component stylesheet so the theme layer cannot spawn a second visual language.
