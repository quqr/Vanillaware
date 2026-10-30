meta:
  id: rsc
  file-extension: rsc
  endian: le
  ks-version: "0.11"
seq:
  - id: entry_count
    type: u4
  - id: checksum
    type: u4
  - id: reserved_08
    size: 8
  - id: entries
    type: entry
    repeat: expr
    repeat-expr: entry_count
types:
  entry:
    seq:
      - id: name
        size: 0x40
      - id: id
        type: u4
      - id: reserved_44
        size: 0x1c
      - id: group
        type: s2
      - id: group_pad
        type: s2
      - id: reserved_64
        size: 0x38
      - id: link
        type: s4
      - id: offset
        type: u4
      - id: index
        type: u4
      - id: flags
        type: u4
      - id: reserved_ac
        type: u4
      - id: kind
        type: u4
