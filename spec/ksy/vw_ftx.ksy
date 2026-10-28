meta:
  id: vw_ftx
  title: Vanillaware FTEX container (Switch)
  file-extension: ftx
  endian: le
  ks-version: "0.11"

doc: |
  结构来源：VanillawareConverter/FTEXConverter/SwitchFtexParser.cs（逐字段等价）。
  真实样本：F:\Codes\13\13rim ex\EX DATA\BattleEffect\Barrier00.ftx（262608 B）
    magic "FTEX" @0x00, version 0x00040140 @0x04, header_size 0x80 @0x08, texture_count 1 @0x0C
    name @0x20 (0x20 B + 0x10 B 保留) = 0x30 B/条
    FTX0 @0x80: nvt_size 0x00040100 @+0x04, nvt_offset 0x40 @+0x08
    NVT  @0x80+0x40: ".tex", format 0x4D(BC7) @+0x04, w 512 @+0x0C, h 512 @+0x10,
                     data_offset 0x100 @+0x1C, data_size 0x40000 @+0x20

  注意：本文件只负责「结构」。像素解码（BC3/BC4/BC7、TegraX1 swizzle、FCMP LZSS）
  在 vw-formats 的 codec 层手写，Kaitai 不承担。

seq:
  - id: magic
    contents: "FTEX"
  - id: version
    type: u4
    doc: 0x00040140 等，含义未解，仅透传
  - id: header_size
    type: u4
    doc: 首个 FTX0 块相对文件起点的偏移
  - id: texture_count
    type: u4
  - id: reserved_10
    size: 0x10
    doc: 0x10..0x20
  - id: entries
    type: texture_entry
    repeat: expr
    repeat-expr: texture_count
  - id: header_padding
    size: header_size - (0x20 + texture_count * 0x30)
    if: header_size > 0x20 + texture_count * 0x30
    doc: 名字表之后到首个块之间的填充
  - id: blocks
    type: ftx_block
    repeat: expr
    repeat-expr: texture_count
    doc: 块按 sz1+sz2 顺序紧挨排列，可顺序读取

types:
  texture_entry:
    seq:
      - id: name
        size: 0x20
        doc: NUL 结尾的 ASCII 文件名，Rust 侧用 from_utf8_lossy 解码
      - id: reserved
        size: 0x10

  ftx_block:
    seq:
      - id: magic
        contents: "FTX0"
      - id: nvt_size
        type: u4
        doc: NVT 数据长度（样本 0x00040100）
      - id: nvt_offset
        type: u4
        doc: NVT 相对块起点的偏移（样本 0x40）
      - id: padding
        size: nvt_offset - 12
        if: nvt_offset > 12
      - id: nvt
        type: nvt
        size: nvt_size

  nvt:
    seq:
      - id: magic
        contents: ".tex"
      - id: format
        type: u2
        doc: 0x44 BC3 / 0x49 BC4 / 0x4D BC7。故意不用 enum：未列出的值会让生成的代码在 try_into() 处报 NotInEnum
      - id: reserved_06
        size: 2
      - id: flags
        type: u4
      - id: width
        type: u4
      - id: height
        type: u4
      - id: reserved_14
        size: 8
      - id: data_offset
        type: u4
        doc: 像素数据相对 .tex 起点的偏移（样本 0x100）
      - id: data_size
        type: u4
        doc: 像素数据长度（样本 0x40000）
      - id: data_padding
        size: data_offset - 0x24
        if: data_offset > 0x24
      - id: data
        size: data_size
