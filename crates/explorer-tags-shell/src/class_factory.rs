use windows::core::{implement, IUnknown, Result, GUID};
use windows::Win32::Foundation::{BOOL, CLASS_E_NOAGGREGATION, E_POINTER};
use windows::Win32::System::Com::IClassFactory_Impl;

pub type CreateInstanceFn = fn(&GUID, *mut *mut std::ffi::c_void) -> Result<()>;

#[implement(windows::Win32::System::Com::IClassFactory)]
pub struct ClassFactory {
    create_fn: CreateInstanceFn,
}

impl ClassFactory {
    pub fn new(create_fn: CreateInstanceFn) -> Self {
        crate::dll_add_ref();
        Self { create_fn }
    }
}

impl Drop for ClassFactory {
    fn drop(&mut self) {
        crate::dll_release();
    }
}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut std::ffi::c_void,
    ) -> Result<()> {
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if ppvobject.is_null() || riid.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe {
            *ppvobject = std::ptr::null_mut();
            (self.create_fn)(&*riid, ppvobject)
        }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            crate::dll_add_ref();
        } else {
            crate::dll_release();
        }
        Ok(())
    }
}
