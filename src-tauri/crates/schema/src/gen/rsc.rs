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
pub struct Rsc {
    pub _root: SharedType<Rsc>,
    pub _parent: SharedType<Rsc>,
    pub _self: SharedType<Self>,
    entry_count: RefCell<u32>,
    checksum: RefCell<u32>,
    reserved_08: RefCell<Vec<u8>>,
    entries: RefCell<Vec<OptRc<Rsc_Entry>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Rsc {
    type Root = Rsc;
    type Parent = Rsc;

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
        *self_rc.entry_count.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.checksum.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_08.borrow_mut() = _io.read_bytes(8 as usize)?.into();
        *self_rc.entries.borrow_mut() = Vec::new();
        let l_entries = *self_rc.entry_count();
        for _i in 0..l_entries {
            let t = Self::read_into::<_, Rsc_Entry>(
                &*_io,
                Some(self_rc._root.clone()),
                Some(self_rc._self.clone()),
            )?
            .into();
            self_rc.entries.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl Rsc {}
impl Rsc {
    pub fn entry_count(&self) -> Ref<'_, u32> {
        self.entry_count.borrow()
    }
}
impl Rsc {
    pub fn checksum(&self) -> Ref<'_, u32> {
        self.checksum.borrow()
    }
}
impl Rsc {
    pub fn reserved_08(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_08.borrow()
    }
}
impl Rsc {
    pub fn entries(&self) -> Ref<'_, Vec<OptRc<Rsc_Entry>>> {
        self.entries.borrow()
    }
}
impl Rsc {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Rsc_Entry {
    pub _root: SharedType<Rsc>,
    pub _parent: SharedType<Rsc>,
    pub _self: SharedType<Self>,
    name: RefCell<Vec<u8>>,
    id: RefCell<u32>,
    reserved_44: RefCell<Vec<u8>>,
    group: RefCell<i16>,
    group_pad: RefCell<i16>,
    reserved_64: RefCell<Vec<u8>>,
    link: RefCell<i32>,
    offset: RefCell<u32>,
    index: RefCell<u32>,
    flags: RefCell<u32>,
    reserved_ac: RefCell<u32>,
    kind: RefCell<u32>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Rsc_Entry {
    type Root = Rsc;
    type Parent = Rsc;

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
        *self_rc.name.borrow_mut() = _io.read_bytes(64 as usize)?.into();
        *self_rc.id.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_44.borrow_mut() = _io.read_bytes(28 as usize)?.into();
        *self_rc.group.borrow_mut() = _io.read_s2le()?.into();
        *self_rc.group_pad.borrow_mut() = _io.read_s2le()?.into();
        *self_rc.reserved_64.borrow_mut() = _io.read_bytes(56 as usize)?.into();
        *self_rc.link.borrow_mut() = _io.read_s4le()?.into();
        *self_rc.offset.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.index.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_ac.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.kind.borrow_mut() = _io.read_u4le()?.into();
        Ok(())
    }
}
impl Rsc_Entry {}
impl Rsc_Entry {
    pub fn name(&self) -> Ref<'_, Vec<u8>> {
        self.name.borrow()
    }
}
impl Rsc_Entry {
    pub fn id(&self) -> Ref<'_, u32> {
        self.id.borrow()
    }
}
impl Rsc_Entry {
    pub fn reserved_44(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_44.borrow()
    }
}
impl Rsc_Entry {
    pub fn group(&self) -> Ref<'_, i16> {
        self.group.borrow()
    }
}
impl Rsc_Entry {
    pub fn group_pad(&self) -> Ref<'_, i16> {
        self.group_pad.borrow()
    }
}
impl Rsc_Entry {
    pub fn reserved_64(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_64.borrow()
    }
}
impl Rsc_Entry {
    pub fn link(&self) -> Ref<'_, i32> {
        self.link.borrow()
    }
}
impl Rsc_Entry {
    pub fn offset(&self) -> Ref<'_, u32> {
        self.offset.borrow()
    }
}
impl Rsc_Entry {
    pub fn index(&self) -> Ref<'_, u32> {
        self.index.borrow()
    }
}
impl Rsc_Entry {
    pub fn flags(&self) -> Ref<'_, u32> {
        self.flags.borrow()
    }
}
impl Rsc_Entry {
    pub fn reserved_ac(&self) -> Ref<'_, u32> {
        self.reserved_ac.borrow()
    }
}
impl Rsc_Entry {
    pub fn kind(&self) -> Ref<'_, u32> {
        self.kind.borrow()
    }
}
impl Rsc_Entry {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
