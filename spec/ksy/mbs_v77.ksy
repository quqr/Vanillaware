meta:
  id: mbs_v77
  title: Vanillaware FMBS/MBS animation container (v72/v76/v77 family)
  file-extension: mbs
  endian: le
  ks-version: "0.11"
params:
  - id: kind
    type: u1
  - id: num_s0
    type: u4
  - id: num_s1
    type: u4
  - id: num_s2
    type: u4
  - id: num_s3
    type: u4
  - id: num_s4
    type: u4
  - id: num_s5
    type: u4
  - id: num_s6
    type: u4
  - id: num_s7
    type: u4
  - id: num_s8
    type: u4
  - id: num_s9
    type: u4
  - id: num_sa
    type: u4
  - id: num_sb
    type: u4

seq:
  - id: magic
    contents: "FMBS"
  - id: header_pad
    size: 0x10
  - id: version
    type: u2

instances:
  s0_sp:
    pos: 0x0b0
    type: u4
  s0_records:
    pos: s0_sp
    type: rec_s0
    repeat: expr
    repeat-expr: num_s0

  s1_sp:
    pos: 0x0b8
    type: u4
  s1_records:
    pos: s1_sp
    type: rec_quad30
    repeat: expr
    repeat-expr: num_s1

  s2_sp:
    pos: 0x0c0
    type: u4
  s2_records:
    pos: s2_sp
    type: rec_quad30
    repeat: expr
    repeat-expr: num_s2

  s3_sp:
    pos: 0x0c8
    type: u4
  s3_records:
    pos: s3_sp
    type: rec_s3
    repeat: expr
    repeat-expr: num_s3

  s4_sp:
    pos: 0x0d0
    type: u4
  s4_records_v72:
    pos: s4_sp
    type: rec_s4_v72
    repeat: expr
    repeat-expr: num_s4
    if: kind == 0
  s4_records_v76:
    pos: s4_sp
    type: rec_s4_v76
    repeat: expr
    repeat-expr: num_s4
    if: kind != 0

  s5_sp:
    pos: 0x0d8
    type: u4
  s5_records:
    pos: s5_sp
    type: rec_s5
    repeat: expr
    repeat-expr: num_s5

  s6_sp:
    pos: 0x0e0
    type: u4
  s6_records:
    pos: s6_sp
    type: rec_s6
    repeat: expr
    repeat-expr: num_s6

  s7_sp:
    pos: 0x0e8
    type: u4
  s7_records:
    pos: s7_sp
    type: rec_s7
    repeat: expr
    repeat-expr: num_s7

  s8_sp:
    pos: 0x0f0
    type: u4
  s8_records:
    pos: s8_sp
    type: rec_s8
    repeat: expr
    repeat-expr: num_s8

  s9_sp:
    pos: 0x0f8
    type: u4
  s9_records:
    pos: s9_sp
    type: rec_s9
    repeat: expr
    repeat-expr: num_s9

  sa_sp:
    pos: 0x100
    type: u4
  sa_records:
    pos: sa_sp
    type: rec_sa
    repeat: expr
    repeat-expr: num_sa

  sb_sp:
    pos: 0x108
    type: u4
  sb_records:
    pos: sb_sp
    type: rec_sb
    repeat: expr
    repeat-expr: num_sb

types:
  rec_s0:
    seq:
      - id: center
        size: 4
      - id: c0
        size: 4
      - id: c1
        size: 4
      - id: c2
        size: 4
      - id: c3
        size: 4
      - id: tail
        size: 4

  rec_quad30:
    seq:
      - id: floats
        type: f4
        repeat: expr
        repeat-expr: 12

  rec_s3:
    seq:
      - id: rect
        type: f4
        repeat: expr
        repeat-expr: 8
      - id: xyz
        type: f4
        repeat: expr
        repeat-expr: 12

  rec_s4_v72:
    seq:
      - id: b00
        type: u4
      - id: color
        type: u1
      - id: flags
        type: u1
      - id: blend
        type: u1
      - id: tex
        type: u1
      - id: attrib
        type: u2
      - id: s1
        type: u2
      - id: s0
        type: u2
      - id: s2
        type: u2

  rec_s4_v76:
    seq:
      - id: b00
        type: u4
      - id: color
        type: u1
      - id: flags
        type: u1
      - id: blend
        type: u1
      - id: tex
        type: u1
      - id: attrib
        type: u4
      - id: s1
        type: u2
      - id: s0
        type: u2
      - id: s2
        type: u2
      - id: tail
        size: 2

  rec_s5:
    seq:
      - id: s3
        type: u2
      - id: b02
        type: u1
      - id: b03
        type: u1
      - id: flags
        type: u4

  rec_s6:
    seq:
      - id: left
        type: f4
      - id: top
        type: f4
      - id: right
        type: f4
      - id: bottom
        type: f4
      - id: s4_set
        type: u4
      - id: s5_set
        type: u2
      - id: s4_no
        type: u2
      - id: s5_no
        type: u1
      - id: flags
        type: u1
      - id: tail
        size: 2

  rec_s7:
    seq:
      - id: move_
        type: f4
        repeat: expr
        repeat-expr: 3
      - id: rotate
        type: f4
        repeat: expr
        repeat-expr: 3
      - id: scale
        type: f4
        repeat: expr
        repeat-expr: 2
      - id: fog
        size: 4

  rec_s8:
    seq:
      - id: s6
        type: u2
      - id: pad_02
        size: 2
      - id: s7
        type: u2
      - id: frames
        type: u2
      - id: flags
        type: u4
      - id: loop_
        type: u2
      - id: in_s5s3
        type: u1
      - id: in_rate
        type: u1
      - id: in_s7
        type: u1
      - id: in_s6
        type: u1
      - id: in_s0s1s2
        type: u1
      - id: b13
        type: u1
      - id: b14
        type: u4
      - id: b18
        type: u2
      - id: sfx_mute
        type: u2
      - id: sfx
        type: u4

  rec_s9:
    seq:
      - id: left
        type: f4
      - id: top
        type: f4
      - id: right
        type: f4
      - id: bottom
        type: f4
      - id: name
        size: 0x18
      - id: sa_set
        type: u2
      - id: sa_no
        type: u1
      - id: sa_main
        type: u1
      - id: sasb_set
        type: u2
      - id: sasb_no
        type: u1
      - id: b2f
        type: u1

  rec_sa:
    seq:
      - id: s8_set
        type: u2
      - id: s8_no
        type: u2
      - id: s8_sum
        type: u4
      - id: s8_sum_once
        type: u4
      - id: b0c
        type: u4
      - id: sb_set
        type: u2
      - id: sb_no
        type: u1
      - id: st
        type: u1
      - id: b14
        type: u2
      - id: tail
        size: 2

  rec_sb:
    seq:
      - id: b00
        type: u4
      - id: b04
        type: u2
      - id: pad_06
        size: 2
      - id: b08
        type: u4
      - id: b0c
        type: u2
      - id: b0e
        type: u2
      - id: b10
        type: u4
