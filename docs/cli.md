# qtstool CLI 使用说明

Vanillaware 文件转换工具的命令行模式。所有命令采用统一的 `-flag` 风格。

## 语法规则

- 命令一律以 `-` 开头：`-tex`、`-sprite`、`-spine`、`-convert`、`-help`
- 选项一律单横线长名：`-input`、`-output`、`-format`、`-game`、`-textures`、`-verbose`
- **第一个不带横线的裸参数自动作为 `-input`**（快捷写法）
- 单操作命令（`-tex`、`-spine`、`-convert`）支持裸路径快捷写法
- `-sprite` 有两个子操作，必须显式指定 `-parse` 或 `-toquad`
- 未知命令或选项会报错并打印对应 help
- 目录批处理默认多线程并行执行

## 命令一览

| 命令 | 说明 |
|------|------|
| `-tex` | 解码 FTEX/FTP 纹理为图片 |
| `-sprite` | Sprite 文件操作（parse / toquad） |
| `-spine` | QUAD JSON 转 Spine 2D |
| `-convert` | 一键转换（自动识别输入类型） |
| `-help` | 显示帮助 |

## -tex 纹理解码

```text
qtstool -tex <path>                                    # 快捷写法
qtstool -tex -decode -input <path> [-output <dir>] [-format png|clut] [-verbose]
```

| 选项 | 说明 |
|------|------|
| `-input <path>` | 输入文件或目录（必需；可为裸参数） |
| `-output <dir>` | 输出目录（默认：input 所在位置，目录输入时保持相对结构） |
| `-format <fmt>` | 输出格式：`png`（默认）或 `clut` |
| `-verbose` | 输出详细日志 |

示例：

```text
qtstool -tex "F:\game\extract\main-ex"
qtstool -tex -decode -input sprite.ftx -output output\
```

## -sprite Sprite 操作

```text
qtstool -sprite -parse <path>                          # MBP/MBS → V55 JSON
qtstool -sprite -toquad <path>                         # MBP/MBS/V55 → QUAD JSON
qtstool -sprite -parse -input sprite.mbs [-output <path>] [-game <id>] [-verbose]
```

| 选项 | 说明 |
|------|------|
| `-input <path>` | 输入 `.mbs`/`.mbp`（toquad 还接受 `.json`/`.v55`） |
| `-output <path>` | 输出文件（默认：`<input>.v55.json` / `<input>.quad.json`） |
| `-game <id>` | 游戏 ID（平台无法自动识别时必需） |
| `-verbose` | 输出详细日志 |

游戏 ID：

```text
ps2_grim  PS2   GrimGrimoire
ps2_odin  PS2   Odin Sphere
nds_kuma  NDS   Kumatanchi
wii_mura  Wii   Muramasa
ps3_drag  PS3   Dragon's Crown
ps3_odin  PS3   Odin Sphere Leifthrasir
ps4_sent  PS4   13 Sentinels
swi_sent  Swit  13 Sentinels
swi_grim  Swit  GrimGrimoire HD
swi_unic  Swit  Unicorn Overlord
ps4_unic  PS4   Unicorn Overlord
```

## -spine QUAD 转 Spine 2D

```text
qtstool -spine <path>
qtstool -spine -input <path> [-output <path>] [-textures <dir>] [-verbose]
```

| 选项 | 说明 |
|------|------|
| `-input <path>` | 输入 QUAD JSON 文件（必需） |
| `-output <path>` | 输出 Spine JSON（默认：`<input>_spine.json`） |
| `-textures <dir>` | 图片纹理目录 |
| `-verbose` | 输出详细日志 |

## -convert 一键转换

自动识别输入类型：`.ftp/.ftx` → 图片，`.mbs/.mbp` → QUAD JSON，`.json` → Spine JSON。输入为目录时递归处理全部支持文件，输出到 `<dir>\output\`（textures/quads 子目录），多线程并行。

```text
qtstool -convert <path>
qtstool -convert -input <path> [-output <dir>] [-game <id>] [-textures <dir>] [-format png|clut] [-verbose]
```

## 帮助

```text
qtstool -help              # 总帮助
qtstool -tex -help         # 各命令详细帮助
qtstool -sprite -parse -help
```

## 返回值

- `0`：全部成功
- `1`：存在失败项或参数错误（错误信息输出到 stderr）
