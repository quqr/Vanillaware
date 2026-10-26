# ADR-0019：统一可视化图元层（FMD / OSB / RSC）

- **状态**：已接受（2026-10-22）
- **影响范围**：`vw-formats::viz`、`src-tauri` 可视化命令、`src/components/viewers`、`src/lib/ipc.ts`
- **补充**：ADR-0018（文档树是检查工具，不是可视化）

## 背景

文档树解决了「结构是否读对」，但没有解决「资源长什么样」：

- FMD 是 3D 模型，只有结构树时用户看不到几何；
- OSB 是场景装配，需要把 FMD/FTX/MBS/FSB 等引用组合起来；
- RSC 是脚本资源包，价值在于跨文件引用与跳转，而不是单文件树。

如果每类资源各自定义一份 DTO 与渲染器，会出现三套坐标、三套引用、三套相机，
后续再组合（OSB 引用 FMD）时无法复用。

## 决策

先定义一层**与格式无关的可视化图元**，再让 FMD/OSB/RSC 各自投影进来：

```text
VizSceneDto {
  bounds: [minX, minY, minZ, maxX, maxY, maxZ],
  nodes: [VizNodeDto],
  references: [VizReferenceDto],
}

VizNodeDto {
  id, parent, name, kind: group | mesh | sprite | light | camera | reference,
  transform: 16 floats,          // 列主序；OSB 的 model/anime 填入 common88 的 TRS
  mesh: VizMeshDto | null,
  meshRef: nodeId | null,        // 复用另一节点的网格（同一 FMD 被多个 model 引用）
  reference: VizReferenceDto | null,
}

VizMeshDto {
  positions: [f32],              // xyz 连续
  indices: [u32],                // 三角列表；FMD 由 sub 的 start/triCount 生成
  uvs: [f32],                    // 每顶点 2 个 f32；空数组表示没有 UV 流
  bounds: [6 f32],
  subRanges: [{ start, count, material, page, rgba, texRef }],
                                 // page: FTX 页下标，-1 表示无贴图
                                 // rgba: 材质 tint；texRef: -1/0/1/2/3 = default/NML/ADD/SUB/OPA
}

VizReferenceDto {
  path, ext, role,               // role: texture | model | animation | audio | script | text
  status: resolved | missing,
}
```

- **引用是一等公民**：OSB 的 FMD/FTX/MBS/FSB、RSC 的 FMSB 都以 `reference` 节点出现，
  前端点击即可跳到对应文件；不把二进制塞进同一个 DTO。
- **FMD 做贴图/纯色三角面 + 线框 + 轨道相机**：positions 与按 sub 生成的 indices
  先验证坐标、包围盒与三角拓扑；UV 取 MESH 的 UV 属性流（V 轴不翻转，glTF 顶左原点，
  Three.js 需 `flipY = false`）。材质通过 `MTRL[K].t2 → FTX 页下标` 映射到
  `subRanges[].page`，前端按页分组建材质并异步解码贴图；缺页或无贴图的段退回纯色。
  `texRef` 决定混合：0 NML 用 `alphaTest` 镂空，1 ADD / 2 SUB 用加/减混合且不写深度，
  3 OPA 与 -1 走不透明；`rgba` 作为 tint 乘到贴图上。无贴图且 alpha 为 0 的碰撞材质
  （BT_STAGE 的 `HIT`）不生成绘制段。OSB 动画/相机事件、骨骼蒙皮留到后续。
- **OSB 复用已有 MBS/FTX 查看器**：`scene` 查看器把 OSB 节点树摊平成图元 + 引用，
  静态摆位先落地，动画/相机事件后接。model/anime 的 `origin/scale/rotate` 取自
  common88；`rotate` 以度存储，按 RotZ → RotY → RotX 组合成节点矩阵。
- **RSC 先做引用图**：`graph` 查看器展示「文件 → 条目 → 字节码载荷」
  （`kind` 为 file / entry / script），不做 3D。RSC 载荷不是 FMSB 魔数文件，
  而是 FMSB VM 字节码，因此按 `entry.offset` 到下一个更大 offset 切段展示。
- **前端 3D 使用 Three.js**：FMD 网格数据量大，CPU 投影或 SVG 会在十万三角级别卡死；
  Three.js 直接上传 buffer，相机/线框/后续材质都有现成实现。

## 代价

- 新增 `three` 前端依赖与一个 WebGL 上下文；
- `VizSceneDto` 需要同时服务 3D、2D 与图结构，字段会比单一查看器更通用；
- 贴图按 `MTRL.t2` 页下标绑定、按 `tex_ref` 近似混合，但仍是静态姿态（未做骨骼蒙皮
  与动画驱动），视觉上不是最终渲染。

## 备选方案

- 每格式一个 DTO/渲染器：短期快，OSB 组合时无法复用。
- 直接用 PixiJS 2D 投影：无新依赖，但大网格 CPU 排序成本高。
- 只做 bbox 占位：实现最快，但用户看不到几何，无法验证解析正确性。

---

## English summary

Define one format-agnostic visualization layer (`VizSceneDto` / `VizNodeDto` / `VizMeshDto` /
`VizReferenceDto`) before adding format-specific renderers. FMD projects mesh positions, sequential
triangle indices and UVs from its sub ranges into a **textured/solid + wireframe + orbit camera** view;
each sub range carries the FTX page index resolved from `MTRL[K].t2` plus the material tint and
render path; the frontend groups materials by page/tint/blend, decodes pages asynchronously, and
falls back to a solid colour when a page is missing. `texRef` drives blending (NML alpha-tested
cutout, ADD/SUB additive/subtractive with no depth write), and zero-alpha collision-only materials
are dropped. Skeletal skinning, animation and OSB camera events are still pending. OSB projects its
node tree into primitives plus first-class references
(model/texture/animation/audio) so the existing MBS/FTX viewers can be reused. RSC starts as a resource
reference graph rather than a 3D view. Three.js is used for FMD because CPU projection/SVG does not
scale to six-figure triangle counts.
