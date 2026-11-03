// This is a generated file! Please edit source .ksy file and use kaitai-struct-compiler to rebuild

#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(irrefutable_let_patterns)]
#![allow(unused_comparisons)]

extern crate kaitai;
use kaitai::*;
use std::cell::{Cell, Ref, RefCell};
use std::convert::{TryFrom, TryInto};
use std::rc::{Rc, Weak};

#[derive(Default, Debug, Clone)]
pub struct Vs2 {
    pub _root: SharedType<Vs2>,
    pub _parent: SharedType<Vs2>,
    pub _self: SharedType<Self>,
    count: RefCell<u32>,
    reserved: RefCell<u32>,
    header_pad: RefCell<Vec<u8>>,
    ids: RefCell<Vec<u32>>,
    masks: RefCell<Vec<OptRc<Vs2_Mask>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Vs2 {
    type Root = Vs2;
    type Parent = Vs2;

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
        *self_rc.count.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.header_pad.borrow_mut() = _io.read_bytes(8 as usize)?.into();
        *self_rc.ids.borrow_mut() = Vec::new();
        let l_ids = *self_rc.count();
        for _i in 0..l_ids {
            self_rc.ids.borrow_mut().push(_io.read_u4le()?.into());
        }
        *self_rc.masks.borrow_mut() = Vec::new();
        let l_masks = *self_rc.count();
        for _i in 0..l_masks {
            let t = Self::read_into::<_, Vs2_Mask>(
                &*_io,
                Some(self_rc._root.clone()),
                Some(self_rc._self.clone()),
            )?
            .into();
            self_rc.masks.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl Vs2 {}
impl Vs2 {
    pub fn count(&self) -> Ref<'_, u32> {
        self.count.borrow()
    }
}
impl Vs2 {
    pub fn reserved(&self) -> Ref<'_, u32> {
        self.reserved.borrow()
    }
}
impl Vs2 {
    pub fn header_pad(&self) -> Ref<'_, Vec<u8>> {
        self.header_pad.borrow()
    }
}
impl Vs2 {
    pub fn ids(&self) -> Ref<'_, Vec<u32>> {
        self.ids.borrow()
    }
}
impl Vs2 {
    pub fn masks(&self) -> Ref<'_, Vec<OptRc<Vs2_Mask>>> {
        self.masks.borrow()
    }
}
impl Vs2 {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Vs2_Mask {
    pub _root: SharedType<Vs2>,
    pub _parent: SharedType<Vs2>,
    pub _self: SharedType<Self>,
    low: RefCell<u32>,
    high: RefCell<u32>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Vs2_Mask {
    type Root = Vs2;
    type Parent = Vs2;

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
        *self_rc.low.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.high.borrow_mut() = _io.read_u4le()?.into();
        Ok(())
    }
}
impl Vs2_Mask {}
impl Vs2_Mask {
    pub fn low(&self) -> Ref<'_, u32> {
        self.low.borrow()
    }
}
impl Vs2_Mask {
    pub fn high(&self) -> Ref<'_, u32> {
        self.high.borrow()
    }
}
impl Vs2_Mask {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
