meta:
  id: ftex_index
  title: Vanillaware FTEX index (Switch) - header only
  file-extension: ftx
  endian: le
  ks-version: "0.11"

doc: |
  Same layout as ftex_switch.ksy but without pixel data: NVT stops at data_offset/data_size.
  用途：帧数/尺寸/格式补全（probe）。整块像素由 Kaitai 读进 Vec<u8> 的话，一个 83MB 的
  Allocating the 83MB payload just to count frames is wasteful, so the field stays empty here; decoding goes through ftex_switch.ksy.

seq:
  - id: magic
    contents: "FTEX"
  - id: version
    type: u4
  - id: header_size
    type: u4
  - id: texture_count
    type: u4
  - id: reserved_10
    size: 0x10
  - id: entries
    type: texture_entry
    repeat: expr
    repeat-expr: texture_count
  - id: header_padding
    size: header_size - (0x20 + texture_count * 0x30)
    if: header_size > 0x20 + texture_count * 0x30
  - id: blocks
    type: ftx_block
    repeat: expr
    repeat-expr: texture_count

types:
  texture_entry:
    seq:
      - id: name
        size: 0x20
      - id: reserved
        size: 0x10

  ftx_block:
    seq:
      - id: magic
        contents: "FTX0"
      - id: nvt_size
        type: u4
      - id: nvt_offset
        type: u4
      - id: padding
        size: nvt_offset - 12
        if: nvt_offset > 12
      - id: nvt
        type: nvt_header
        size: nvt_size

  nvt_header:
    seq:
      - id: magic
        contents: ".tex"
      - id: format
        type: u2
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
      - id: data_size
        type: u4
      # 到此为止：不解析 data
