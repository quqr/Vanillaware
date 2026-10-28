# 0022 — 补收 13 侧的 OEB / ORB / BST：它们与 OSB 是同一个容器

## 状态

已采纳（2026-10-28）。

## 背景

ADR-0020 之后注册表里 26 个扩展名全部闭合，但那是「已登记的都闭合」，
不是「语料里的原生资源都登记了」。把两份游戏的解包目录按扩展名盘一遍后，
发现 13 侧还有三类带魔数、结构可读、却完全不在注册表里的资源：

| 扩展名 | 数量 | 目录 | 魔数 | 内容 |
|---|---|---|---|---|
| `.oeb` | 107 | `EX DATA/MobArrange` | `OEB\0` | NPC 摆位（谁站在哪里） |
| `.orb` | 15 | `EX DATA/Round` | `ORB\0` | 轮次镜头表 + 用到的舞台/背景清单 |
| `.bst` | 43 | `EX DATA/BattleStageData` | `BBST` | 战斗关卡数据 |

被过滤掉的其余扩展名都是派生产物或工具链文件：`.png`/`.json`/`.gltf`/`.obj`/`.quad`
（导出），`.lnk`（解包工具留的快捷方式），`.c`/`.py`/`.bat`/`.sample`（开发物），
`.otf`/`.ttf`（字体，按既有决定不收）。

## 决策

### 1. `.oeb` / `.orb` 与 `.osb` 共用同一个容器

一开始把它们当成「未知格式」，按 `type + 0x52` 猜记录步长，怎么都对不上。
转折点是注意到 `.osb` 的节点头就是 `type(u32) + bodySize(u32) + childCount(u32) + pad`，
拿这套规则往 `.oeb` / `.orb` 上试：

- OEB **107/107**、ORB **15/15** 精确闭合到 EOF；
- 节点 body 的前 0x52 字节与 OSB 同构：`caption[0x40]`（Shift-JIS）+ `label[0x10]` + `group u16`。

于是抽出一个共享模块 `doc/bundle.rs`：头部 + 资源路径池 + 通用节点树，
各 bundle 只提供自己的「节点类型编号 → 名字」表。`.osb` 保留原有实现
（它有按类型解码的字段语义），`.oeb` / `.orb` 走通用容器。

### 2. 节点类型的含义靠 caption 反推

| 文件 | 类型表 |
|---|---|
| OEB | `4 arrange_root` / `26 group` / `27 entry` / `28 slot` |
| ORB | `1 round_tree` / `29 round` / `30 camera` |

OEB 的 `slot`（type 28）body 固定 228 字节，`0x52` 起是 `scaleX, scaleY, x, y, z`
五个 f32；caption 是「女子3 (鞄) 待機」这类描述，label 是角色编号（`JK03B`）。
ORB 的 `camera`（type 30）caption 是舞台名（「新校舎廊下」），`0x52` 起是三个 f32 位置。

### 3. `.bst` 只做到「名字 + 标识符」

它不是 bundle 族：`BBST` + 12 字节 0 + 条目数（116～118）+ 名字区。
名字之间夹的字段宽度随名字长度变（4～5 字节），`+0x16` 的「名字区长度」只覆盖到第一个名字，
所以按「扫可打印串 + 已知扩展名」取资源名；条目区结构没有闭合（试过多种步长与偏移），
文档树只报头部、资源名、大写标识符（`ENE_RANDOMWAVEGEN` 之类），数据按字节保留。

### 4. `.bin` 仍然不收

`Other/Battle*List.bin` 确实是原生列表，但语料里绝大多数 `.bin`
（`BattleEffect` / `BattleStageModel` / `Unit` / `bg`）都是 `fmd_extract.py v2`
导出的 glTF 缓冲，同目录必有同名 `.gltf`。按「只收原生资源」的规则继续排除，
理由写进注册表注释，避免下次又翻一遍。

## 后果

- 注册表从 26 个扩展到 **29 个**扩展名；13 侧的原生资源全部可见。
- 内容视图新增三种：OEB 的站位表 + 俯视散点图（实测 177 个角色，看得出成排分布）、
  ORB 的镜头表 + 195 条完整资产路径、BST 的资源与标识符表。
- `bundle.closed`（资源池 + 节点树恰好到 EOF）成为 OSB 族的新不变量，
  OEB 107/107、ORB 15/15 全通过。
- `.bst` 的条目结构仍是空白，后续要解需要从执行侧入手。

## 验收入口

```bash
cargo run -p vw-formats --example doc_probe  -- <files…>   # 结构树 + 全部 check
cargo run -p vw-media  --example content_probe -- <files…>  # 内容视图（VW_DUMP_DIR 可落盘位图）
```
