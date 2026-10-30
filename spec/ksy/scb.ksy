meta:
  id: scb
  file-extension: scb
  endian: le
  ks-version: "0.11"
seq:
  - id: token_count
    type: u4
  - id: tokens
    type: token
    repeat: expr
    repeat-expr: token_count
types:
  token:
    seq:
      - id: len
        type: u4
      - id: raw
        size: len
