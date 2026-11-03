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
pub struct RscCommand {
    pub _root: SharedType<RscCommand>,
    pub _parent: SharedType<RscCommand>,
    pub _self: SharedType<Self>,
    commands: RefCell<Vec<OptRc<RscCommand_Command>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for RscCommand {
    type Root = RscCommand;
    type Parent = RscCommand;

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
        *self_rc.commands.borrow_mut() = Vec::new();
        {
            let mut _i = 0;
            while !_io.is_eof() {
                let t = Self::read_into::<_, RscCommand_Command>(
                    &*_io,
                    Some(self_rc._root.clone()),
                    Some(self_rc._self.clone()),
                )?
                .into();
                self_rc.commands.borrow_mut().push(t);
                _i += 1;
            }
        }
        Ok(())
    }
}
impl RscCommand {}
impl RscCommand {
    pub fn commands(&self) -> Ref<'_, Vec<OptRc<RscCommand_Command>>> {
        self.commands.borrow()
    }
}
impl RscCommand {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct RscCommand_Command {
    pub _root: SharedType<RscCommand>,
    pub _parent: SharedType<RscCommand>,
    pub _self: SharedType<Self>,
    opcode: RefCell<u32>,
    flags: RefCell<u32>,
    body_size: RefCell<u32>,
    serial: RefCell<u32>,
    body: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for RscCommand_Command {
    type Root = RscCommand;
    type Parent = RscCommand;

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
        *self_rc.opcode.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.flags.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.body_size.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.serial.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.body.borrow_mut() = _io.read_bytes(*self_rc.body_size() as usize)?.into();
        Ok(())
    }
}
impl RscCommand_Command {}
impl RscCommand_Command {
    pub fn opcode(&self) -> Ref<'_, u32> {
        self.opcode.borrow()
    }
}
impl RscCommand_Command {
    pub fn flags(&self) -> Ref<'_, u32> {
        self.flags.borrow()
    }
}
impl RscCommand_Command {
    pub fn body_size(&self) -> Ref<'_, u32> {
        self.body_size.borrow()
    }
}
impl RscCommand_Command {
    pub fn serial(&self) -> Ref<'_, u32> {
        self.serial.borrow()
    }
}
impl RscCommand_Command {
    pub fn body(&self) -> Ref<'_, Vec<u8>> {
        self.body.borrow()
    }
}
impl RscCommand_Command {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
