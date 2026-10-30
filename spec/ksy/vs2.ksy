meta:
  id: vs2
  file-extension: vs2
  endian: le
  ks-version: "0.11"
seq:
  - id: count
    type: u4
  - id: reserved
    type: u4
  - id: header_pad
    size: 8
  - id: ids
    type: u4
    repeat: expr
    repeat-expr: count
  - id: masks
    type: mask
    repeat: expr
    repeat-expr: count
types:
  mask:
    seq:
      - id: low
        type: u4
      - id: high
        type: u4
