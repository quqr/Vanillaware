// This is a generated file! Please edit source .ksy file and use kaitai-struct-compiler to rebuild

#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(irrefutable_let_patterns)]
#![allow(unused_comparisons)]

extern crate kaitai;
use kaitai::*;
use std::convert::{TryFrom, TryInto};
use std::cell::{Ref, Cell, RefCell};
use std::rc::{Rc, Weak};

/**
 * 结构来源（真值）：rip-vanillaware/FMBP_FMBS_vanillaware.php（Rufas Wan，PHP），
 * 分节表读法见同目录 vanillaware.inc 的 $gp_data['ver72'|'ver76'|'ver77']。
 * 
 * MBS 不是顺序结构：
 *   1. 从分节表给出的 sp_at 处读 4 字节 -> 该节数据的绝对偏移 sp；
 *   2. 从 count_at 处读 count_bytes 字节 -> 记录条数；
 *   3. 按固定步长读记录；记录之间靠索引互引（s4->s0/s1/s2，s5->s3，s6->s4/s5）。
 * 
 * 分节表（sp_at / count_at / count_bytes / stride）的权威落地是
 * crates/formats/src/mbs/profile.rs，它逐行抄自 PHP 的 $gp_data 表。
 * 本文件只描述「记录长什么样」：s0..sb 的 sp 位置三版本一致（0x0b0 + 8*i），
 * 而 count 的位置与宽度随版本变化，故由调用方（Rust 侧读 profile.rs）
 * 通过 params 传入，避免在 ksy 里复刻版本分支。
 * 
 * 之所以不把 count 写成带 switch-on 的 instance：ksc 0.11 的 rust 后端不支持
 * 「repeat + switch-on」的类型级分派（会漏生成枚举定义，且 repeat-expr 拿到枚举
 * 而非整数），生成的代码无法编译。故统一走 params。
 * 
 * 像素 / 图集解码（内嵌 FTEX、BC 解码、swizzle）不进本文件，走 formats 的 codec 层。
 */

#[derive(Default, Debug, Clone)]
pub struct MbsV77 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    kind: RefCell<u8>,
    num_s0: RefCell<u32>,
    num_s1: RefCell<u32>,
    num_s2: RefCell<u32>,
    num_s3: RefCell<u32>,
    num_s4: RefCell<u32>,
    num_s5: RefCell<u32>,
    num_s6: RefCell<u32>,
    num_s7: RefCell<u32>,
    num_s8: RefCell<u32>,
    num_s9: RefCell<u32>,
    num_sa: RefCell<u32>,
    num_sb: RefCell<u32>,
    magic: RefCell<Vec<u8>>,
    header_pad: RefCell<Vec<u8>>,
    version: RefCell<u16>,
    _io: RefCell<BytesReader>,
    f_s0_records: Cell<bool>,
    s0_records: RefCell<Vec<OptRc<MbsV77_RecS0>>>,
    f_s0_sp: Cell<bool>,
    s0_sp: RefCell<u32>,
    f_s1_records: Cell<bool>,
    s1_records: RefCell<Vec<OptRc<MbsV77_RecQuad30>>>,
    f_s1_sp: Cell<bool>,
    s1_sp: RefCell<u32>,
    f_s2_records: Cell<bool>,
    s2_records: RefCell<Vec<OptRc<MbsV77_RecQuad30>>>,
    f_s2_sp: Cell<bool>,
    s2_sp: RefCell<u32>,
    f_s3_records: Cell<bool>,
    s3_records: RefCell<Vec<OptRc<MbsV77_RecS3>>>,
    f_s3_sp: Cell<bool>,
    s3_sp: RefCell<u32>,
    f_s4_records_v72: Cell<bool>,
    s4_records_v72: RefCell<Vec<OptRc<MbsV77_RecS4V72>>>,
    f_s4_records_v76: Cell<bool>,
    s4_records_v76: RefCell<Vec<OptRc<MbsV77_RecS4V76>>>,
    f_s4_sp: Cell<bool>,
    s4_sp: RefCell<u32>,
    f_s5_records: Cell<bool>,
    s5_records: RefCell<Vec<OptRc<MbsV77_RecS5>>>,
    f_s5_sp: Cell<bool>,
    s5_sp: RefCell<u32>,
    f_s6_records: Cell<bool>,
    s6_records: RefCell<Vec<OptRc<MbsV77_RecS6>>>,
    f_s6_sp: Cell<bool>,
    s6_sp: RefCell<u32>,
    f_s7_records: Cell<bool>,
    s7_records: RefCell<Vec<OptRc<MbsV77_RecS7>>>,
    f_s7_sp: Cell<bool>,
    s7_sp: RefCell<u32>,
    f_s8_records: Cell<bool>,
    s8_records: RefCell<Vec<OptRc<MbsV77_RecS8>>>,
    f_s8_sp: Cell<bool>,
    s8_sp: RefCell<u32>,
    f_s9_records: Cell<bool>,
    s9_records: RefCell<Vec<OptRc<MbsV77_RecS9>>>,
    f_s9_sp: Cell<bool>,
    s9_sp: RefCell<u32>,
    f_sa_records: Cell<bool>,
    sa_records: RefCell<Vec<OptRc<MbsV77_RecSa>>>,
    f_sa_sp: Cell<bool>,
    sa_sp: RefCell<u32>,
    f_sb_records: Cell<bool>,
    sb_records: RefCell<Vec<OptRc<MbsV77_RecSb>>>,
    f_sb_sp: Cell<bool>,
    sb_sp: RefCell<u32>,
}
impl KStruct for MbsV77 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.magic.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        if !(*self_rc.magic() == vec![0x46u8, 0x4du8, 0x42u8, 0x53u8]) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::NotEqual, src_path: "/seq/0".to_string() }));
        }
        *self_rc.header_pad.borrow_mut() = _io.read_bytes(16 as usize)?.into();
        *self_rc.version.borrow_mut() = _io.read_u2le()?.into();
        Ok(())
    }
}
impl MbsV77 {
    pub fn kind(&self) -> Ref<'_, u8> {
        self.kind.borrow()
    }
}
impl MbsV77 {
    pub fn num_s0(&self) -> Ref<'_, u32> {
        self.num_s0.borrow()
    }
}
impl MbsV77 {
    pub fn num_s1(&self) -> Ref<'_, u32> {
        self.num_s1.borrow()
    }
}
impl MbsV77 {
    pub fn num_s2(&self) -> Ref<'_, u32> {
        self.num_s2.borrow()
    }
}
impl MbsV77 {
    pub fn num_s3(&self) -> Ref<'_, u32> {
        self.num_s3.borrow()
    }
}
impl MbsV77 {
    pub fn num_s4(&self) -> Ref<'_, u32> {
        self.num_s4.borrow()
    }
}
impl MbsV77 {
    pub fn num_s5(&self) -> Ref<'_, u32> {
        self.num_s5.borrow()
    }
}
impl MbsV77 {
    pub fn num_s6(&self) -> Ref<'_, u32> {
        self.num_s6.borrow()
    }
}
impl MbsV77 {
    pub fn num_s7(&self) -> Ref<'_, u32> {
        self.num_s7.borrow()
    }
}
impl MbsV77 {
    pub fn num_s8(&self) -> Ref<'_, u32> {
        self.num_s8.borrow()
    }
}
impl MbsV77 {
    pub fn num_s9(&self) -> Ref<'_, u32> {
        self.num_s9.borrow()
    }
}
impl MbsV77 {
    pub fn num_sa(&self) -> Ref<'_, u32> {
        self.num_sa.borrow()
    }
}
impl MbsV77 {
    pub fn num_sb(&self) -> Ref<'_, u32> {
        self.num_sb.borrow()
    }
}
impl MbsV77 {
    pub fn set_params(&mut self, kind: u8, num_s0: u32, num_s1: u32, num_s2: u32, num_s3: u32, num_s4: u32, num_s5: u32, num_s6: u32, num_s7: u32, num_s8: u32, num_s9: u32, num_sa: u32, num_sb: u32) {
        *self.kind.borrow_mut() = kind;
        *self.num_s0.borrow_mut() = num_s0;
        *self.num_s1.borrow_mut() = num_s1;
        *self.num_s2.borrow_mut() = num_s2;
        *self.num_s3.borrow_mut() = num_s3;
        *self.num_s4.borrow_mut() = num_s4;
        *self.num_s5.borrow_mut() = num_s5;
        *self.num_s6.borrow_mut() = num_s6;
        *self.num_s7.borrow_mut() = num_s7;
        *self.num_s8.borrow_mut() = num_s8;
        *self.num_s9.borrow_mut() = num_s9;
        *self.num_sa.borrow_mut() = num_sa;
        *self.num_sb.borrow_mut() = num_sb;
    }
}
impl MbsV77 {
    pub fn s0_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS0>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s0_records.get() {
            return Ok(self.s0_records.borrow());
        }
        self.f_s0_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s0_sp()? as usize)?;
        *self.s0_records.borrow_mut() = Vec::new();
        let l_s0_records = *self.num_s0();
        for _i in 0..l_s0_records {
            let t = Self::read_into::<_, MbsV77_RecS0>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s0_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s0_records.borrow())
    }
    pub fn s0_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s0_sp.get() {
            return Ok(self.s0_sp.borrow());
        }
        self.f_s0_sp.set(true);
        let _pos = _io.pos();
        _io.seek(176 as usize)?;
        *self.s0_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s0_sp.borrow())
    }
    pub fn s1_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecQuad30>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s1_records.get() {
            return Ok(self.s1_records.borrow());
        }
        self.f_s1_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s1_sp()? as usize)?;
        *self.s1_records.borrow_mut() = Vec::new();
        let l_s1_records = *self.num_s1();
        for _i in 0..l_s1_records {
            let t = Self::read_into::<_, MbsV77_RecQuad30>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s1_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s1_records.borrow())
    }
    pub fn s1_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s1_sp.get() {
            return Ok(self.s1_sp.borrow());
        }
        self.f_s1_sp.set(true);
        let _pos = _io.pos();
        _io.seek(184 as usize)?;
        *self.s1_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s1_sp.borrow())
    }
    pub fn s2_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecQuad30>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s2_records.get() {
            return Ok(self.s2_records.borrow());
        }
        self.f_s2_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s2_sp()? as usize)?;
        *self.s2_records.borrow_mut() = Vec::new();
        let l_s2_records = *self.num_s2();
        for _i in 0..l_s2_records {
            let t = Self::read_into::<_, MbsV77_RecQuad30>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s2_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s2_records.borrow())
    }
    pub fn s2_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s2_sp.get() {
            return Ok(self.s2_sp.borrow());
        }
        self.f_s2_sp.set(true);
        let _pos = _io.pos();
        _io.seek(192 as usize)?;
        *self.s2_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s2_sp.borrow())
    }
    pub fn s3_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS3>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s3_records.get() {
            return Ok(self.s3_records.borrow());
        }
        self.f_s3_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s3_sp()? as usize)?;
        *self.s3_records.borrow_mut() = Vec::new();
        let l_s3_records = *self.num_s3();
        for _i in 0..l_s3_records {
            let t = Self::read_into::<_, MbsV77_RecS3>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s3_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s3_records.borrow())
    }
    pub fn s3_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s3_sp.get() {
            return Ok(self.s3_sp.borrow());
        }
        self.f_s3_sp.set(true);
        let _pos = _io.pos();
        _io.seek(200 as usize)?;
        *self.s3_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s3_sp.borrow())
    }
    pub fn s4_records_v72(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS4V72>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s4_records_v72.get() {
            return Ok(self.s4_records_v72.borrow());
        }
        self.f_s4_records_v72.set(true);
        if ((*self.kind() as u8) == (0 as u8)) {
            let _pos = _io.pos();
            _io.seek(*self.s4_sp()? as usize)?;
            *self.s4_records_v72.borrow_mut() = Vec::new();
            let l_s4_records_v72 = *self.num_s4();
            for _i in 0..l_s4_records_v72 {
                let t = Self::read_into::<_, MbsV77_RecS4V72>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
                self.s4_records_v72.borrow_mut().push(t);
            }
            _io.seek(_pos)?;
        }
        Ok(self.s4_records_v72.borrow())
    }
    pub fn s4_records_v76(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS4V76>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s4_records_v76.get() {
            return Ok(self.s4_records_v76.borrow());
        }
        self.f_s4_records_v76.set(true);
        if ((*self.kind() as u8) != (0 as u8)) {
            let _pos = _io.pos();
            _io.seek(*self.s4_sp()? as usize)?;
            *self.s4_records_v76.borrow_mut() = Vec::new();
            let l_s4_records_v76 = *self.num_s4();
            for _i in 0..l_s4_records_v76 {
                let t = Self::read_into::<_, MbsV77_RecS4V76>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
                self.s4_records_v76.borrow_mut().push(t);
            }
            _io.seek(_pos)?;
        }
        Ok(self.s4_records_v76.borrow())
    }
    pub fn s4_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s4_sp.get() {
            return Ok(self.s4_sp.borrow());
        }
        self.f_s4_sp.set(true);
        let _pos = _io.pos();
        _io.seek(208 as usize)?;
        *self.s4_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s4_sp.borrow())
    }
    pub fn s5_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS5>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s5_records.get() {
            return Ok(self.s5_records.borrow());
        }
        self.f_s5_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s5_sp()? as usize)?;
        *self.s5_records.borrow_mut() = Vec::new();
        let l_s5_records = *self.num_s5();
        for _i in 0..l_s5_records {
            let t = Self::read_into::<_, MbsV77_RecS5>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s5_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s5_records.borrow())
    }
    pub fn s5_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s5_sp.get() {
            return Ok(self.s5_sp.borrow());
        }
        self.f_s5_sp.set(true);
        let _pos = _io.pos();
        _io.seek(216 as usize)?;
        *self.s5_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s5_sp.borrow())
    }
    pub fn s6_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS6>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s6_records.get() {
            return Ok(self.s6_records.borrow());
        }
        self.f_s6_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s6_sp()? as usize)?;
        *self.s6_records.borrow_mut() = Vec::new();
        let l_s6_records = *self.num_s6();
        for _i in 0..l_s6_records {
            let t = Self::read_into::<_, MbsV77_RecS6>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s6_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s6_records.borrow())
    }
    pub fn s6_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s6_sp.get() {
            return Ok(self.s6_sp.borrow());
        }
        self.f_s6_sp.set(true);
        let _pos = _io.pos();
        _io.seek(224 as usize)?;
        *self.s6_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s6_sp.borrow())
    }
    pub fn s7_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS7>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s7_records.get() {
            return Ok(self.s7_records.borrow());
        }
        self.f_s7_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s7_sp()? as usize)?;
        *self.s7_records.borrow_mut() = Vec::new();
        let l_s7_records = *self.num_s7();
        for _i in 0..l_s7_records {
            let t = Self::read_into::<_, MbsV77_RecS7>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s7_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s7_records.borrow())
    }
    pub fn s7_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s7_sp.get() {
            return Ok(self.s7_sp.borrow());
        }
        self.f_s7_sp.set(true);
        let _pos = _io.pos();
        _io.seek(232 as usize)?;
        *self.s7_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s7_sp.borrow())
    }
    pub fn s8_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS8>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s8_records.get() {
            return Ok(self.s8_records.borrow());
        }
        self.f_s8_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s8_sp()? as usize)?;
        *self.s8_records.borrow_mut() = Vec::new();
        let l_s8_records = *self.num_s8();
        for _i in 0..l_s8_records {
            let t = Self::read_into::<_, MbsV77_RecS8>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s8_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s8_records.borrow())
    }
    pub fn s8_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s8_sp.get() {
            return Ok(self.s8_sp.borrow());
        }
        self.f_s8_sp.set(true);
        let _pos = _io.pos();
        _io.seek(240 as usize)?;
        *self.s8_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s8_sp.borrow())
    }
    pub fn s9_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecS9>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s9_records.get() {
            return Ok(self.s9_records.borrow());
        }
        self.f_s9_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.s9_sp()? as usize)?;
        *self.s9_records.borrow_mut() = Vec::new();
        let l_s9_records = *self.num_s9();
        for _i in 0..l_s9_records {
            let t = Self::read_into::<_, MbsV77_RecS9>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.s9_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.s9_records.borrow())
    }
    pub fn s9_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_s9_sp.get() {
            return Ok(self.s9_sp.borrow());
        }
        self.f_s9_sp.set(true);
        let _pos = _io.pos();
        _io.seek(248 as usize)?;
        *self.s9_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.s9_sp.borrow())
    }
    pub fn sa_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecSa>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_sa_records.get() {
            return Ok(self.sa_records.borrow());
        }
        self.f_sa_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.sa_sp()? as usize)?;
        *self.sa_records.borrow_mut() = Vec::new();
        let l_sa_records = *self.num_sa();
        for _i in 0..l_sa_records {
            let t = Self::read_into::<_, MbsV77_RecSa>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.sa_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.sa_records.borrow())
    }
    pub fn sa_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_sa_sp.get() {
            return Ok(self.sa_sp.borrow());
        }
        self.f_sa_sp.set(true);
        let _pos = _io.pos();
        _io.seek(256 as usize)?;
        *self.sa_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.sa_sp.borrow())
    }
    pub fn sb_records(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<MbsV77_RecSb>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_sb_records.get() {
            return Ok(self.sb_records.borrow());
        }
        self.f_sb_records.set(true);
        let _pos = _io.pos();
        _io.seek(*self.sb_sp()? as usize)?;
        *self.sb_records.borrow_mut() = Vec::new();
        let l_sb_records = *self.num_sb();
        for _i in 0..l_sb_records {
            let t = Self::read_into::<_, MbsV77_RecSb>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.sb_records.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.sb_records.borrow())
    }
    pub fn sb_sp(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_sb_sp.get() {
            return Ok(self.sb_sp.borrow());
        }
        self.f_sb_sp.set(true);
        let _pos = _io.pos();
        _io.seek(264 as usize)?;
        *self.sb_sp.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.sb_sp.borrow())
    }
}
impl MbsV77 {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}

/**
 * 0x04..0x14，PHP 不读
 */
impl MbsV77 {
    pub fn header_pad(&self) -> Ref<'_, Vec<u8>> {
        self.header_pad.borrow()
    }
}

/**
 * 0x72 / 0x76 / 0x77，仅用于自校验
 */
impl MbsV77 {
    pub fn version(&self) -> Ref<'_, u16> {
        self.version.borrow()
    }
}
impl MbsV77 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecQuad30 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    floats: RefCell<Vec<f32>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecQuad30 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.floats.borrow_mut() = Vec::new();
        let l_floats = 12;
        for _i in 0..l_floats {
            self_rc.floats.borrow_mut().push(_io.read_f4le()?.into());
        }
        Ok(())
    }
}
impl MbsV77_RecQuad30 {
}
impl MbsV77_RecQuad30 {
    pub fn floats(&self) -> Ref<'_, Vec<f32>> {
        self.floats.borrow()
    }
}
impl MbsV77_RecQuad30 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS0 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    center: RefCell<Vec<u8>>,
    c0: RefCell<Vec<u8>>,
    c1: RefCell<Vec<u8>>,
    c2: RefCell<Vec<u8>>,
    c3: RefCell<Vec<u8>>,
    tail: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS0 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.center.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        *self_rc.c0.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        *self_rc.c1.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        *self_rc.c2.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        *self_rc.c3.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        *self_rc.tail.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        Ok(())
    }
}
impl MbsV77_RecS0 {
}
impl MbsV77_RecS0 {
    pub fn center(&self) -> Ref<'_, Vec<u8>> {
        self.center.borrow()
    }
}
impl MbsV77_RecS0 {
    pub fn c0(&self) -> Ref<'_, Vec<u8>> {
        self.c0.borrow()
    }
}
impl MbsV77_RecS0 {
    pub fn c1(&self) -> Ref<'_, Vec<u8>> {
        self.c1.borrow()
    }
}
impl MbsV77_RecS0 {
    pub fn c2(&self) -> Ref<'_, Vec<u8>> {
        self.c2.borrow()
    }
}
impl MbsV77_RecS0 {
    pub fn c3(&self) -> Ref<'_, Vec<u8>> {
        self.c3.borrow()
    }
}
impl MbsV77_RecS0 {
    pub fn tail(&self) -> Ref<'_, Vec<u8>> {
        self.tail.borrow()
    }
}
impl MbsV77_RecS0 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS3 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    rect: RefCell<Vec<f32>>,
    xyz: RefCell<Vec<f32>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS3 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.rect.borrow_mut() = Vec::new();
        let l_rect = 8;
        for _i in 0..l_rect {
            self_rc.rect.borrow_mut().push(_io.read_f4le()?.into());
        }
        *self_rc.xyz.borrow_mut() = Vec::new();
        let l_xyz = 12;
        for _i in 0..l_xyz {
            self_rc.xyz.borrow_mut().push(_io.read_f4le()?.into());
        }
        Ok(())
    }
}
impl MbsV77_RecS3 {
}
impl MbsV77_RecS3 {
    pub fn rect(&self) -> Ref<'_, Vec<f32>> {
        self.rect.borrow()
    }
}
impl MbsV77_RecS3 {
    pub fn xyz(&self) -> Ref<'_, Vec<f32>> {
        self.xyz.borrow()
    }
}
impl MbsV77_RecS3 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS4V72 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    b00: RefCell<u32>,
    color: RefCell<u8>,
    flags: RefCell<u8>,
    blend: RefCell<u8>,
    tex: RefCell<u8>,
    attrib: RefCell<u16>,
    s1: RefCell<u16>,
    s0: RefCell<u16>,
    s2: RefCell<u16>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS4V72 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.b00.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.color.borrow_mut() = _io.read_u1()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u1()?.into();
        *self_rc.blend.borrow_mut() = _io.read_u1()?.into();
        *self_rc.tex.borrow_mut() = _io.read_u1()?.into();
        *self_rc.attrib.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s1.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s0.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s2.borrow_mut() = _io.read_u2le()?.into();
        Ok(())
    }
}
impl MbsV77_RecS4V72 {
}
impl MbsV77_RecS4V72 {
    pub fn b00(&self) -> Ref<'_, u32> {
        self.b00.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn color(&self) -> Ref<'_, u8> {
        self.color.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn flags(&self) -> Ref<'_, u8> {
        self.flags.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn blend(&self) -> Ref<'_, u8> {
        self.blend.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn tex(&self) -> Ref<'_, u8> {
        self.tex.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn attrib(&self) -> Ref<'_, u16> {
        self.attrib.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn s1(&self) -> Ref<'_, u16> {
        self.s1.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn s0(&self) -> Ref<'_, u16> {
        self.s0.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn s2(&self) -> Ref<'_, u16> {
        self.s2.borrow()
    }
}
impl MbsV77_RecS4V72 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS4V76 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    b00: RefCell<u32>,
    color: RefCell<u8>,
    flags: RefCell<u8>,
    blend: RefCell<u8>,
    tex: RefCell<u8>,
    attrib: RefCell<u32>,
    s1: RefCell<u16>,
    s0: RefCell<u16>,
    s2: RefCell<u16>,
    tail: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS4V76 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.b00.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.color.borrow_mut() = _io.read_u1()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u1()?.into();
        *self_rc.blend.borrow_mut() = _io.read_u1()?.into();
        *self_rc.tex.borrow_mut() = _io.read_u1()?.into();
        *self_rc.attrib.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.s1.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s0.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s2.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.tail.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        Ok(())
    }
}
impl MbsV77_RecS4V76 {
}
impl MbsV77_RecS4V76 {
    pub fn b00(&self) -> Ref<'_, u32> {
        self.b00.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn color(&self) -> Ref<'_, u8> {
        self.color.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn flags(&self) -> Ref<'_, u8> {
        self.flags.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn blend(&self) -> Ref<'_, u8> {
        self.blend.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn tex(&self) -> Ref<'_, u8> {
        self.tex.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn attrib(&self) -> Ref<'_, u32> {
        self.attrib.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn s1(&self) -> Ref<'_, u16> {
        self.s1.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn s0(&self) -> Ref<'_, u16> {
        self.s0.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn s2(&self) -> Ref<'_, u16> {
        self.s2.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn tail(&self) -> Ref<'_, Vec<u8>> {
        self.tail.borrow()
    }
}
impl MbsV77_RecS4V76 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS5 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    s3: RefCell<u16>,
    b02: RefCell<u8>,
    b03: RefCell<u8>,
    flags: RefCell<u32>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS5 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.s3.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.b02.borrow_mut() = _io.read_u1()?.into();
        *self_rc.b03.borrow_mut() = _io.read_u1()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u4le()?.into();
        Ok(())
    }
}
impl MbsV77_RecS5 {
}
impl MbsV77_RecS5 {
    pub fn s3(&self) -> Ref<'_, u16> {
        self.s3.borrow()
    }
}
impl MbsV77_RecS5 {
    pub fn b02(&self) -> Ref<'_, u8> {
        self.b02.borrow()
    }
}
impl MbsV77_RecS5 {
    pub fn b03(&self) -> Ref<'_, u8> {
        self.b03.borrow()
    }
}
impl MbsV77_RecS5 {
    pub fn flags(&self) -> Ref<'_, u32> {
        self.flags.borrow()
    }
}
impl MbsV77_RecS5 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS6 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    left: RefCell<f32>,
    top: RefCell<f32>,
    right: RefCell<f32>,
    bottom: RefCell<f32>,
    s4_set: RefCell<u32>,
    s5_set: RefCell<u16>,
    s4_no: RefCell<u16>,
    s5_no: RefCell<u8>,
    flags: RefCell<u8>,
    tail: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS6 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.left.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.top.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.right.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.bottom.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.s4_set.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.s5_set.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s4_no.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s5_no.borrow_mut() = _io.read_u1()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u1()?.into();
        *self_rc.tail.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        Ok(())
    }
}
impl MbsV77_RecS6 {
}
impl MbsV77_RecS6 {
    pub fn left(&self) -> Ref<'_, f32> {
        self.left.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn top(&self) -> Ref<'_, f32> {
        self.top.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn right(&self) -> Ref<'_, f32> {
        self.right.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn bottom(&self) -> Ref<'_, f32> {
        self.bottom.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn s4_set(&self) -> Ref<'_, u32> {
        self.s4_set.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn s5_set(&self) -> Ref<'_, u16> {
        self.s5_set.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn s4_no(&self) -> Ref<'_, u16> {
        self.s4_no.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn s5_no(&self) -> Ref<'_, u8> {
        self.s5_no.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn flags(&self) -> Ref<'_, u8> {
        self.flags.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn tail(&self) -> Ref<'_, Vec<u8>> {
        self.tail.borrow()
    }
}
impl MbsV77_RecS6 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS7 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    move_: RefCell<Vec<f32>>,
    rotate: RefCell<Vec<f32>>,
    scale: RefCell<Vec<f32>>,
    fog: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS7 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.move_.borrow_mut() = Vec::new();
        let l_move_ = 3;
        for _i in 0..l_move_ {
            self_rc.move_.borrow_mut().push(_io.read_f4le()?.into());
        }
        *self_rc.rotate.borrow_mut() = Vec::new();
        let l_rotate = 3;
        for _i in 0..l_rotate {
            self_rc.rotate.borrow_mut().push(_io.read_f4le()?.into());
        }
        *self_rc.scale.borrow_mut() = Vec::new();
        let l_scale = 2;
        for _i in 0..l_scale {
            self_rc.scale.borrow_mut().push(_io.read_f4le()?.into());
        }
        *self_rc.fog.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        Ok(())
    }
}
impl MbsV77_RecS7 {
}
impl MbsV77_RecS7 {
    pub fn move_(&self) -> Ref<'_, Vec<f32>> {
        self.move_.borrow()
    }
}
impl MbsV77_RecS7 {
    pub fn rotate(&self) -> Ref<'_, Vec<f32>> {
        self.rotate.borrow()
    }
}
impl MbsV77_RecS7 {
    pub fn scale(&self) -> Ref<'_, Vec<f32>> {
        self.scale.borrow()
    }
}
impl MbsV77_RecS7 {
    pub fn fog(&self) -> Ref<'_, Vec<u8>> {
        self.fog.borrow()
    }
}
impl MbsV77_RecS7 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS8 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    s6: RefCell<u16>,
    pad_02: RefCell<Vec<u8>>,
    s7: RefCell<u16>,
    frames: RefCell<u16>,
    flags: RefCell<u32>,
    loop_: RefCell<u16>,
    in_s5s3: RefCell<u8>,
    in_rate: RefCell<u8>,
    in_s7: RefCell<u8>,
    in_s6: RefCell<u8>,
    in_s0s1s2: RefCell<u8>,
    b13: RefCell<u8>,
    b14: RefCell<u32>,
    b18: RefCell<u16>,
    sfx_mute: RefCell<u16>,
    sfx: RefCell<u32>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS8 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.s6.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.pad_02.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        *self_rc.s7.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.frames.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.loop_.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.in_s5s3.borrow_mut() = _io.read_u1()?.into();
        *self_rc.in_rate.borrow_mut() = _io.read_u1()?.into();
        *self_rc.in_s7.borrow_mut() = _io.read_u1()?.into();
        *self_rc.in_s6.borrow_mut() = _io.read_u1()?.into();
        *self_rc.in_s0s1s2.borrow_mut() = _io.read_u1()?.into();
        *self_rc.b13.borrow_mut() = _io.read_u1()?.into();
        *self_rc.b14.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.b18.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.sfx_mute.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.sfx.borrow_mut() = _io.read_u4le()?.into();
        Ok(())
    }
}
impl MbsV77_RecS8 {
}
impl MbsV77_RecS8 {
    pub fn s6(&self) -> Ref<'_, u16> {
        self.s6.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn pad_02(&self) -> Ref<'_, Vec<u8>> {
        self.pad_02.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn s7(&self) -> Ref<'_, u16> {
        self.s7.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn frames(&self) -> Ref<'_, u16> {
        self.frames.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn flags(&self) -> Ref<'_, u32> {
        self.flags.borrow()
    }
}

/**
 * 回跳目标 s8 索引（`loop` 是 Rust 关键字）
 */
impl MbsV77_RecS8 {
    pub fn loop_(&self) -> Ref<'_, u16> {
        self.loop_.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn in_s5s3(&self) -> Ref<'_, u8> {
        self.in_s5s3.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn in_rate(&self) -> Ref<'_, u8> {
        self.in_rate.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn in_s7(&self) -> Ref<'_, u8> {
        self.in_s7.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn in_s6(&self) -> Ref<'_, u8> {
        self.in_s6.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn in_s0s1s2(&self) -> Ref<'_, u8> {
        self.in_s0s1s2.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn b13(&self) -> Ref<'_, u8> {
        self.b13.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn b14(&self) -> Ref<'_, u32> {
        self.b14.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn b18(&self) -> Ref<'_, u16> {
        self.b18.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn sfx_mute(&self) -> Ref<'_, u16> {
        self.sfx_mute.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn sfx(&self) -> Ref<'_, u32> {
        self.sfx.borrow()
    }
}
impl MbsV77_RecS8 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecS9 {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    left: RefCell<f32>,
    top: RefCell<f32>,
    right: RefCell<f32>,
    bottom: RefCell<f32>,
    name: RefCell<Vec<u8>>,
    sa_set: RefCell<u16>,
    sa_no: RefCell<u8>,
    sa_main: RefCell<u8>,
    sasb_set: RefCell<u16>,
    sasb_no: RefCell<u8>,
    b2f: RefCell<u8>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecS9 {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.left.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.top.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.right.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.bottom.borrow_mut() = _io.read_f4le()?.into();
        *self_rc.name.borrow_mut() = _io.read_bytes(24 as usize)?.into();
        *self_rc.sa_set.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.sa_no.borrow_mut() = _io.read_u1()?.into();
        *self_rc.sa_main.borrow_mut() = _io.read_u1()?.into();
        *self_rc.sasb_set.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.sasb_no.borrow_mut() = _io.read_u1()?.into();
        *self_rc.b2f.borrow_mut() = _io.read_u1()?.into();
        Ok(())
    }
}
impl MbsV77_RecS9 {
}
impl MbsV77_RecS9 {
    pub fn left(&self) -> Ref<'_, f32> {
        self.left.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn top(&self) -> Ref<'_, f32> {
        self.top.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn right(&self) -> Ref<'_, f32> {
        self.right.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn bottom(&self) -> Ref<'_, f32> {
        self.bottom.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn name(&self) -> Ref<'_, Vec<u8>> {
        self.name.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn sa_set(&self) -> Ref<'_, u16> {
        self.sa_set.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn sa_no(&self) -> Ref<'_, u8> {
        self.sa_no.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn sa_main(&self) -> Ref<'_, u8> {
        self.sa_main.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn sasb_set(&self) -> Ref<'_, u16> {
        self.sasb_set.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn sasb_no(&self) -> Ref<'_, u8> {
        self.sasb_no.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn b2f(&self) -> Ref<'_, u8> {
        self.b2f.borrow()
    }
}
impl MbsV77_RecS9 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecSa {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    s8_set: RefCell<u16>,
    s8_no: RefCell<u16>,
    s8_sum: RefCell<u32>,
    s8_sum_once: RefCell<u32>,
    b0c: RefCell<u32>,
    sb_set: RefCell<u16>,
    sb_no: RefCell<u8>,
    st: RefCell<u8>,
    b14: RefCell<u16>,
    tail: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecSa {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.s8_set.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s8_no.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.s8_sum.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.s8_sum_once.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.b0c.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.sb_set.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.sb_no.borrow_mut() = _io.read_u1()?.into();
        *self_rc.st.borrow_mut() = _io.read_u1()?.into();
        *self_rc.b14.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.tail.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        Ok(())
    }
}
impl MbsV77_RecSa {
}
impl MbsV77_RecSa {
    pub fn s8_set(&self) -> Ref<'_, u16> {
        self.s8_set.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn s8_no(&self) -> Ref<'_, u16> {
        self.s8_no.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn s8_sum(&self) -> Ref<'_, u32> {
        self.s8_sum.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn s8_sum_once(&self) -> Ref<'_, u32> {
        self.s8_sum_once.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn b0c(&self) -> Ref<'_, u32> {
        self.b0c.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn sb_set(&self) -> Ref<'_, u16> {
        self.sb_set.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn sb_no(&self) -> Ref<'_, u8> {
        self.sb_no.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn st(&self) -> Ref<'_, u8> {
        self.st.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn b14(&self) -> Ref<'_, u16> {
        self.b14.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn tail(&self) -> Ref<'_, Vec<u8>> {
        self.tail.borrow()
    }
}
impl MbsV77_RecSa {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct MbsV77_RecSb {
    pub _root: SharedType<MbsV77>,
    pub _parent: SharedType<MbsV77>,
    pub _self: SharedType<Self>,
    b00: RefCell<u32>,
    b04: RefCell<u16>,
    pad_06: RefCell<Vec<u8>>,
    b08: RefCell<u32>,
    b0c: RefCell<u16>,
    b0e: RefCell<u16>,
    b10: RefCell<u32>,
    _io: RefCell<BytesReader>,
}
impl KStruct for MbsV77_RecSb {
    type Root = MbsV77;
    type Parent = MbsV77;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.b00.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.b04.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.pad_06.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        *self_rc.b08.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.b0c.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.b0e.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.b10.borrow_mut() = _io.read_u4le()?.into();
        Ok(())
    }
}
impl MbsV77_RecSb {
}
impl MbsV77_RecSb {
    pub fn b00(&self) -> Ref<'_, u32> {
        self.b00.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn b04(&self) -> Ref<'_, u16> {
        self.b04.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn pad_06(&self) -> Ref<'_, Vec<u8>> {
        self.pad_06.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn b08(&self) -> Ref<'_, u32> {
        self.b08.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn b0c(&self) -> Ref<'_, u16> {
        self.b0c.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn b0e(&self) -> Ref<'_, u16> {
        self.b0e.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn b10(&self) -> Ref<'_, u32> {
        self.b10.borrow()
    }
}
impl MbsV77_RecSb {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
