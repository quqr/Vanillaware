meta:
  id: fms
  file-extension: fms
  endian: le
  ks-version: "0.11"
seq:
  - id: magic
    contents: "FMSB"
  - id: payload_size
    type: u4
  - id: header_size
    type: u4
  - id: reserved_0c
    size: 8
  - id: string_count
    type: u4
  - id: encoding
    type: u1
  - id: reserved_19
    size: header_size - 0x19
  - id: slots
    size: string_count * 8
  - id: strings
    type: text
    repeat: expr
    repeat-expr: string_count
  - id: tail
    size-eos: true
types:
  text:
    seq:
      - id: value
        type: str
        encoding: UTF-8
        terminator: 0
