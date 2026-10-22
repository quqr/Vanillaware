# ADR-0004：MBS 的真值来源是 PHP 工具，而非 QuadToSpine2D 的 C# 移植

- **状态**：已接受（2026-10-20）
- **影响范围**：MBS 分节表、V55 模型、Quad 转换、校验基准

## 背景

同一件事有三份实现：

1. `rip-vanillaware/FMBP_FMBS_vanillaware.php` + `v55_to_quad.php`（Rufas Wan，PHP）—— 原始权威。
2. `QuadToSpine2D/VanillawareConverter/MBSConverter/*`（C#）—— 二手移植。
3. 本项目（Rust）—— 待建。

本轮核查发现 **C# 与 PHP 在常量表上已经不一致**：

- `vanilla_blendmode`：`ps2_grim` / `ps2_odin` 分支 PHP 产出 6 条，C# 只产出 3 条。
- `vanilla_mixing`：`nds_kuma` PHP 全 0，C# 走 LINEAR。
- 分节表 `PlatformConfigs.cs` 与 PHP 的 `mbs-data/*.mbs.txt` 的条目数、步长亦须逐条核对，不可假定一致。

C# 是「能跑」的实现，PHP 是「对」的实现。二者冲突时以 PHP 为准。

## 决策

- **真值权威 = PHP**。Rust 解析、V55 建模、Quad 转换的每一个字段、每一张常量表都逐行对齐 PHP。
  禁止从 C# 反推，也禁止自行推导（用户已明确纠正过一次）。
- **校验基准 = PHP 产物**：`F:\Codes\UNICORN OVERLORD\extract\Main-Output\quads\*.quad.json`（722 个）。
- **平台范围 = v77 + v76 + v72 家族**（Switch/PS4 高清系：Unicorn Overlord / 13 Sentinels / Odin / Dragon）。
  ksy 名为 `vw_mbs_v77`，在 Rust 侧以 `tag` 参数泛化三套分节表。
- 分节表数据源：`rip-vanillaware/mbs-data/` 下的 `<tag>.mbs.txt`（分节表）+ PHP 内的 blend/mix switch。
  Rust 把这两类都固化为**数据**（而非散落的 match 分支），便于逐条对照。

## 权衡

代价：PHP 的分节表是「每平台一份文本 + 一段 switch」，转成 Rust 数据后仍有手工搬运的出错空间；因此校验必须逐字段做，不能只看「跑通」。

收益：与唯一权威对齐，后续任何分歧都能回到 PHP 一句话验证；避免继承 C# 的历史 bug。

## 备选方案

- **以 C# 为蓝本移植**：上手快（现成 C# 代码结构清晰），但已证实常量表有错，会把错误带进来。
- **自行推导格式**：在 FTEX 上已因此栽过一次（用户明确否定），不再考虑。

---

## English summary

MBS has three implementations; the PHP tool is authoritative, the C# port is secondary and already
diverges (blend table 6 vs 3 entries for ps2 tags; mixing LINEAR vs all-zero for nds). Every Rust field
and constant table must mirror PHP line-by-line; the C# code must not be used as a blueprint. Golden
baseline = the 722 `.quad.json` produced by PHP. Platform scope = v77 + v76 + v72 family, with the section
table and blend/mix tables stored as data (sourced from `mbs-data/*.mbs.txt` and PHP's switch blocks).
