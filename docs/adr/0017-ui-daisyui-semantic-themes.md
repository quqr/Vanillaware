# ADR-0017：UI 全面使用 daisyUI 语义类与内置主题

- **状态**：已接受（2026-10-22）
- **影响范围**：`src/styles.css`、`src/**/*.tsx`、`src/store/appStore.ts`、`index.html`
- **取代**：ADR-0010（工业暗色 + 细线框，禁用 daisyUI 语义类）

## 背景

ADR-0010 为了与 QuadToSpine2D 拉开距离，选择手写纯 CSS 令牌，并明确不再使用 daisyUI 语义类。
代价是每个交互态（焦点、按压、禁用、校验）都要自己补齐，且增删主题等于重写一套颜色令牌。
仓库已经安装 daisyUI 5 与 Tailwind CSS 4，但 `styles.css` 没有 `@plugin "daisyui"`，组件库实际从未生效；
同时自定义的 `.btn/.badge/.input/.select/.tab` 与 daisyUI 同名，无法平滑共存。

## 决策

- **唯一 UI 层**：Tailwind CSS 4 + daisyUI 5。组件只用 daisyUI 类，布局只用 Tailwind 工具类，颜色只用 daisyUI 语义色。
- **零自定义 CSS**：`src/styles.css` 仅保留 `@import "tailwindcss"` 与 `@plugin "daisyui"` 配置。
  运行期动态值（虚拟列表位移、时间轴百分比、画布棋盘格）以行内样式表达，不写样式表组件类。
- **内置主题**：启用 `dark`（默认）、`light`、`corporate`、`business`、`nord`、`dim`、`night`、`dracula`。
  不做自定义主题；新增主题必须同时更新 `styles.css`、`appStore.THEMES` 与 `index.html` 白名单。
- **主题切换**：工具栏使用 `dropdown` + `menu` + `theme-controller` 单选；选择写入 `localStorage["vw-theme"]`，
  并在首屏脚本中先于应用执行，避免闪烁。
- **视觉语言**：采用 daisyUI 默认外观（默认圆角、`--depth`、标准组件形态），放弃 ADR-0010 的直角细线框。

## 权衡

代价：失去工业暗色的强识别度；八套内置主题意味着颜色搭配由 daisyUI 决定，无法逐主题微调。
收益：交互态、焦点环、禁用态、表单校验由组件库统一提供；配色随主题自动推导，新增主题是配置而非重写 CSS；
代码里不再出现固定色值，亮色主题天然可用。

## 备选方案

- **自定义亮/暗两套主题**：识别度最高，但等于维护第二套设计系统，且只有两种选择。
- **保留手写令牌并叠加 daisyUI**：同名类冲突，必须长期维护覆盖规则，违背「零自定义 CSS」。

---

## English summary

daisyUI 5 + Tailwind CSS 4 become the **only** UI layer. `styles.css` keeps nothing but the Tailwind import and
daisyUI plugin config — zero custom component CSS. Eight built-in themes are enabled (`dark` default, plus light,
corporate, business, nord, dim, night, dracula), switched through a `theme-controller` dropdown and persisted in
`localStorage["vw-theme"]`, with a pre-boot script to avoid a theme flash. The industrial square-corner look of
ADR-0010 is dropped in favor of daisyUI's default appearance. Trade-off: less distinctive branding, but
interaction states and theme-aware colors come from the component library instead of a second design system.
