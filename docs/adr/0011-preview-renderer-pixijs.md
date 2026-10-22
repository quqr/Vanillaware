# ADR-0011：预览渲染层用 PixiJS 8，播放由 ticker 驱动

- **状态**：已接受（2026-10-21）
- **影响范围**：`src/lib/pixiScene.ts`（新）、`src/lib/sceneDraw.ts`、`src/views/PreviewView.tsx`

## 背景

预览器最初用手写 Canvas 2D 做合成：每层按 dstquad 拆成两个三角形，各自构造仿射矩阵、`clip` 后
`drawImage`。这条路在一天内就出了两个层级不同的 bug：

1. 三角形内用了 `setTransform`，把 DPR 与相机变换一起**覆盖**掉，贴图落到裸屏幕坐标上；
2. 清屏按 CSS 像素算，HiDPI 下右下角留残影。

根因不是某一行写错，而是「DPR × 相机 × 纹理像素」三段变换全靠手算矩阵拼装，任何一处顺序错都会
静默变成「画面上看不到东西」。用户明确要求改用 PixiJS。

## 决策

- **渲染层 = PixiJS 8（`pixi.js@^8`）**，`Application` 挂到现有 `<canvas>`，不引入 React 绑定库。
- **每个图层一个 `Mesh`**：`MeshGeometry` 用 4 个顶点 + 索引 `[0,1,2, 0,2,3]`；UV 用**纹理像素坐标**
  （Pixi 默认 UV 矩阵按 `1/width`、`1/height` 缩放），因此 `srcquad × 图集宽高` 直接喂进去，
  不再自己算仿射。
- **相机 = 父 `Container`**：`position = 画布中心 + pan`、`scale = zoom`；图层顶点恒为世界坐标。
  缩放/平移只改这一个节点的变换。
- **播放由 `app.ticker` 驱动**：时间累计、倍速、`loopId` 回跳都在 ticker 回调里完成；
  React 只在 10Hz 节流下限更新帧号/时间码等 HUD，60Hz 不触发重渲染。
- **调试层用 `Graphics`**：命中框、四边形描边、原点十字，线宽按 `1/zoom` 换算，保证屏幕上恒定粗细。
- **纹理来源不变**：仍走 `get_atlas_png`（PNG + 磁盘缓存），前端 `createImageBitmap` 后
  `Texture.from`，按 `texId` 缓存，场景切换时统一 `destroy`。
- **mesh 池**：按需增长、超出帧数的隐藏，上限 1024；不为每帧新建对象。

## 权衡

代价：引入约 300KB（gzip 后约 90KB）的渲染库；WebView2 下强制 `preference: "webgl"`，
没有 Canvas 2D 回退路径（v8 已移除 Canvas 渲染器），显卡不可用时预览会直接报错。

收益：变换由场景图负责，不再手拼矩阵；UV/纹理/裁剪交给 GPU；后续做混合模式、滤镜
（如命中框高亮、颜色通道调试）有现成入口。

## 备选方案

- **继续手写 Canvas 2D**：已经证明三段变换容易出错，且每加一个调试层都要再写一套裁剪逻辑。
- **`@pixi/react`**：多一层 React 绑定，播放又要在 React 生命周期里跑，和「ticker 驱动」冲突。
- **保留双通路（Canvas 2D 回退）**：两套像素通路会带来「有时对有时错」的隐性 bug，ADR-0007 已拒绝同类思路。

---

## English summary

The preview renderer moves to PixiJS 8. Each layer is a `Mesh` with a 4-vertex `MeshGeometry` and two
triangles; UVs are supplied in **texture pixel space** (Pixi's default UV matrix divides by texture size), so no
affine math is hand-rolled. The camera is a parent `Container` (position = canvas centre + pan, scale = zoom),
and all vertex positions stay in world space. Playback is advanced by `app.ticker`; React only receives a 10 Hz
throttled HUD update. Debug overlays (hitboxes, quad outlines, origin cross) are `Graphics` with `1/zoom` line
widths. Textures still come from `get_atlas_png` (PNG + disk cache) and are decoded once per `texId`. A mesh pool
reuses instances and caps at 1024. There is no Canvas 2D fallback — v8 dropped the Canvas renderer — so a missing
WebGL context surfaces as an explicit error rather than a second pixel path.
