meta:
  id: rsc_command
  endian: le
  ks-version: "0.11"
seq:
  - id: commands
    type: command
    repeat: eos
types:
  command:
    seq:
      - id: opcode
        type: u4
      - id: flags
        type: u4
      - id: body_size
        type: u4
      - id: serial
        type: u4
      - id: body
        size: body_size
