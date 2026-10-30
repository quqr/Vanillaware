meta:
  id: ftex_switch
  title: Vanillaware FTEX container (Switch)
  file-extension: ftx
  endian: le
  ks-version: "0.11"
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
        type: nvt
        size: nvt_size

  nvt:
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
      - id: data_padding
        size: data_offset - 0x24
        if: data_offset > 0x24
      - id: data
        size: data_size
