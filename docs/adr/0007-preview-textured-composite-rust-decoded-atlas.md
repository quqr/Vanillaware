# ADR-0007：预览器做贴图合成，纹理由 Rust 自行从 mbs 内嵌数据解码

- **状态**：已接受（2026-10-20）
- **影响范围**：预览视图、`vw-formats` 的图集解码、缓存

## 背景

要让动画「看起来对」，必须把 `srcquad` 的 UV 映射到真实像素上，再按 `dstquad` 摆到画布，
并按 `blend_id` / mix 处理混合与插值。线框只能验几何，验不了采样与混合。

现状有两条纹理通路：

1. PHP 已把 mbs 内嵌纹理解成图集 PNG，落在 `Main-Output/textures/<quad名>.<tex_id>.png`（2031 个）。
2. mbs 自身内嵌着压缩纹理，可由既有 FTEX 解码链（BC3/BC4/BC7 + swizzle）自解。

直接吃现成 PNG 最快，但会让预览器依赖外部产物的存在与命名——应用自身不自足。

## 决策

- **预览器做贴图合成**（非线框）：按 dstquad/srcquad/blend_id 渲染。
- **纹理由 Rust 自行解码**：从 `.mbs` 内嵌数据取图集，走 `vw-formats` 既有 FTEX 解码链，
  不依赖 `Main-Output/textures` 的外部 PNG。
  - 现实约束：现有 FTEX 链是 Switch（v77）路径；PS4（v76/v72 家族的 ps4 侧）需要对应平台的
    FTEX 解码分支，若该分支尚未就绪，预览对 ps4 侧可先降级为「几何 + 占位纹理」并明确标注，
    不得静默显示错像素。
- 交互（全部纳入）：
  - 播放控制：多动画列表、播放/暂停/循环、速度倍率；
  - 时间轴：指针可拖拽、帧号/时间码显示、逐帧步进；
  - 画布：缩放/平移、背景网格或棋盘、坐标轴与回心点；
  - 图层与调试：skeleton / slot / dstquad / hitquad 图层开关，单 quad 高亮与属性面板。
- 合成结果进内存缓存，键含 `(mbs路径, tex_id, cacheEpoch)`；沿用既有缩略图缓存范式。

## 权衡

代价：自解码把 FTEX 平台分支的缺口暴露到预览路径上（ps4 侧可能先降级）；且需要为图集单独做缓存。

收益：应用自足，不依赖 `Main-Output` 这类外部工具产物；纹理与几何走同一条解码链，不会出现「两套像素」。

## 备选方案

- **只用现成 PNG**：快，但把工具的可用性绑定在外部产物上。
- **纯线框**：省事，但验不了采样/混合，等于放弃「动画预览」的意义。
- **现成 PNG 优先 + 缺失回退自解码**：双通路，反而是两倍维护面；且两条通路的像素若不一致会产生
  「有时对有时错」的隐性 bug。

---

## 修订（2026-10-21）：纹理容器的归属规则

实测 `000` 树的 725 个 `.mbs`，纹理容器有两种命名并存：357 个**同名**（`BGBtAbandonedPort_B.mbs`
↔ `BGBtAbandonedPort_B.ftx`），198 个**同名 + 数字后缀**（`Fighter_M.mbs` ↔ `Fighter_M00.ftx`），
2 个两种都有（`Eltlinde_F` 有 `00` 与 `10`），165 个没有纹理。

因此 `atlas::sibling_container` 的规则是：**同名优先，否则取「同名 + 纯数字」的唯一候选**；
候选多于一个时返回 `Unsupported` 而不是任选其一。预览界面上表现为「图集缺该帧」的错误提示，
符合「宁可报缺，也不画错像素」的既定约束。`tests/atlas_index.rs` 用 PHP 落盘的 `<容器名>.<i>.png`
数量交叉核对帧数口径（抽样 24 个，实测 16 个命中、1 个因归属歧义跳过）。

## English summary

The preview composites textures (dstquad/srcquad/blend_id), not wireframes. Texture atlases are decoded by
Rust from data embedded in the `.mbs`, through the existing FTEX decode chain, rather than depending on the
externally generated `Main-Output/textures/*.png`. If the PS4-side FTEX branch is not yet ready, PS4 preview
degrades to geometry plus a clearly-labelled placeholder texture — never silently wrong pixels. All four
interaction families (playback, timeline, canvas navigation, layer/debug) are in scope. Composite results are
cached in memory keyed by (mbs path, tex_id, cacheEpoch).

Revision: container resolution prefers an exact-stem `.ftx`/`.ftp`, then the unique `<stem><digits>` variant;
more than one candidate is an error instead of a guess. `tests/atlas_index.rs` cross-checks frame counts
against the PHP-produced `<container>.<i>.png` naming.
