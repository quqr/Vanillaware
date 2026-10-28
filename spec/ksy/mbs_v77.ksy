meta:
  id: mbs_v77
  title: Vanillaware FMBS/MBS animation container (v72/v76/v77 family)
  file-extension: mbs
  endian: le
  ks-version: "0.11"

doc: |
  结构来源（真值）：rip-vanillaware/FMBP_FMBS_vanillaware.php（Rufas Wan，PHP），
  分节表读法见同目录 vanillaware.inc 的 $gp_data['ver72'|'ver76'|'ver77']。

  MBS 不是顺序结构：
    1. 从分节表给出的 sp_at 处读 4 字节 -> 该节数据的绝对偏移 sp；
    2. 从 count_at 处读 count_bytes 字节 -> 记录条数；
    3. 按固定步长读记录；记录之间靠索引互引（s4->s0/s1/s2，s5->s3，s6->s4/s5）。

  分节表（sp_at / count_at / count_bytes / stride）的权威落地是
  crates/formats/src/mbs/profile.rs，它逐行抄自 PHP 的 $gp_data 表。
  本文件只描述「记录长什么样」：s0..sb 的 sp 位置三版本一致（0x0b0 + 8*i），
  而 count 的位置与宽度随版本变化，故由调用方（Rust 侧读 profile.rs）
  通过 params 传入，避免在 ksy 里复刻版本分支。

  之所以不把 count 写成带 switch-on 的 instance：ksc 0.11 的 rust 后端不支持
  「repeat + switch-on」的类型级分派（会漏生成枚举定义，且 repeat-expr 拿到枚举
  而非整数），生成的代码无法编译。故统一走 params。

  像素 / 图集解码（内嵌 FTEX、BC 解码、swizzle）不进本文件，走 formats 的 codec 层。

params:
  - id: kind
    type: u1
    doc: |
      0 = v72 家族（ps3_odin / ps4_odin / ps4_drag）
      1 = v76 家族（ps4_sent）
      2 = v77 家族（swi_sent / swi_grim / swi_unic / ps4_grim / ps4_unic / ps5_* / xbx_unic）
      仅用于区分 s4 的记录步长：v72 为 0x10，v76/v77 为 0x14。
  - id: num_s0
    type: u4
    doc: s0 记录条数
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
    doc: 0x04..0x14，PHP 不读
  - id: version
    type: u2
    doc: 0x72 / 0x76 / 0x77，仅用于自校验

instances:
  # sp 的位置三版本一致（0x0b0 + 8*i），可直接写死。
  # ---- s0：4 个 RGBA 颜色（nds_quad18c），步长 0x18 ----
  s0_sp:
    pos: 0x0b0
    type: u4
  s0_records:
    pos: s0_sp
    type: rec_s0
    repeat: expr
    repeat-expr: num_s0

  # ---- s1：12 个 f4（nds_quad30p），步长 0x30 ----
  s1_sp:
    pos: 0x0b8
    type: u4
  s1_records:
    pos: s1_sp
    type: rec_quad30
    repeat: expr
    repeat-expr: num_s1

  # ---- s2：12 个 f4（nds_quad30p），步长 0x30 ----
  s2_sp:
    pos: 0x0c0
    type: u4
  s2_records:
    pos: s2_sp
    type: rec_quad30
    repeat: expr
    repeat-expr: num_s2

  # ---- s3：rect(8 f4) + xyz(12 f4)，步长 0x50 ----
  s3_sp:
    pos: 0x0c8
    type: u4
  s3_records:
    pos: s3_sp
    type: rec_s3
    repeat: expr
    repeat-expr: num_s3

  # ---- s4：层记录；v72 步长 0x10，v76/v77 步长 0x14 ----
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

  # ---- s5：判定框引用，步长 0x08 ----
  s5_sp:
    pos: 0x0d8
    type: u4
  s5_records:
    pos: s5_sp
    type: rec_s5
    repeat: expr
    repeat-expr: num_s5

  # ---- s6：帧矩形 + s4/s5 区间，步长 0x1c ----
  s6_sp:
    pos: 0x0e0
    type: u4
  s6_records:
    pos: s6_sp
    type: rec_s6
    repeat: expr
    repeat-expr: num_s6

  # ---- s7：变换 + 雾色，步长 0x24 ----
  s7_sp:
    pos: 0x0e8
    type: u4
  s7_records:
    pos: s7_sp
    type: rec_s7
    repeat: expr
    repeat-expr: num_s7

  # ---- s8：时间线条目，步长 0x20 ----
  s8_sp:
    pos: 0x0f0
    type: u4
  s8_records:
    pos: s8_sp
    type: rec_s8
    repeat: expr
    repeat-expr: num_s8

  # ---- s9：具名矩形 + sa/b 区间，步长 0x30 ----
  s9_sp:
    pos: 0x0f8
    type: u4
  s9_records:
    pos: s9_sp
    type: rec_s9
    repeat: expr
    repeat-expr: num_s9

  # ---- sa：s8 区间，步长 0x18 ----
  sa_sp:
    pos: 0x100
    type: u4
  sa_records:
    pos: sa_sp
    type: rec_sa
    repeat: expr
    repeat-expr: num_sa

  # ---- sb：次要区间，步长 0x14 ----
  sb_sp:
    pos: 0x108
    type: u4
  sb_records:
    pos: sb_sp
    type: rec_sb
    repeat: expr
    repeat-expr: num_sb

types:
  # s0：中心 + 4 颜色 + 尾部填充（nds_quad18c 取 offset 4/8/c/10 的 4 字节 RGBA）
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

  # s1 / s2：12 个 f4，PHP nds_quad30p 取下标 2..9
  rec_quad30:
    seq:
      - id: floats
        type: f4
        repeat: expr
        repeat-expr: 12

  # s3：rect(x1,y1,x2,y2,x3,y3,x4,y4) + xyz(4 x n)
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

  # s4（v72）：步长 0x10
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

  # s4（v76 / v77）：步长 0x14
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

  # s5：s3 索引 + flags
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

  # s6：帧矩形 + s4/s5 引用区间
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

  # s7：move / rotate / scale + 雾色 4 字节（`move` 是 Rust 关键字，故记作 move_）
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

  # s8：时间线条目
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
        doc: 回跳目标 s8 索引（`loop` 是 Rust 关键字）
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

  # s9：具名矩形 + sa/b 引用区间
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

  # sa：s8 引用区间（字段止于 0x16，尾部 2 字节填充凑满步长 0x18）
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

  # sb：次要区间
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
