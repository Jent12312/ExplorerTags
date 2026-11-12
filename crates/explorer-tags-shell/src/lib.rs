#![allow(non_snake_case)]

pub mod class_factory;
pub mod menu_command;
pub mod property_handler;
pub mod registration;

use std::sync::atomic::{AtomicIsize, Ordering};
use class_factory::ClassFactory;
use menu_command::MainMenuCommand;
use property_handler::PropertyHandler;
use explorer_tags_core::guids::{CLSID_EXPLORER_TAGS_MENU, CLSID_EXPLORER_TAGS_PROP_HANDLER};
use windows::core::{Interface, IUnknown, GUID, HRESULT};
use windows::Win32::Foundation::{
    BOOL, CLASS_E_CLASSNOTAVAILABLE, E_POINTER, HINSTANCE, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;

static DLL_REF_COUNT: AtomicIsize = AtomicIsize::new(0);
static mut HMODULE_INSTANCE: HINSTANCE = HINSTANCE(std::ptr::null_mut());

pub fn dll_add_ref() {
    DLL_REF_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn dll_release() {
    DLL_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
}

pub fn get_hinstance() -> HMODULE {
    let mut hmodule = HMODULE::default();
    unsafe {
        let _ = windows::Win32::System::LibraryLoader::GetModuleHandleExW(
            windows::Win32::System::LibraryLoader::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                | windows::Win32::System::LibraryLoader::GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            windows::core::PCWSTR(get_hinstance as *const () as _),
            &mut hmodule,
        );
    }
    hmodule
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hinst: HINSTANCE,
    reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> BOOL {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        HMODULE_INSTANCE = hinst;
        DisableThreadLibraryCalls(HMODULE(hinst.0)).ok();
    }
    BOOL(1)
}

#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REF_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

fn create_menu_command(riid: &GUID, ppv: *mut *mut std::ffi::c_void) -> windows::core::Result<()> {
    let cmd: IUnknown = MainMenuCommand::new().into();
    unsafe { cmd.query(riid, ppv).ok() }
}

fn create_property_handler(riid: &GUID, ppv: *mut *mut std::ffi::c_void) -> windows::core::Result<()> {
    let handler: IUnknown = PropertyHandler::new().into();
    unsafe { handler.query(riid, ppv).ok() }
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut std::ffi::c_void,
) -> HRESULT {
    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    let clsid = *rclsid;
    let factory: IClassFactory = if clsid == CLSID_EXPLORER_TAGS_MENU {
        ClassFactory::new(create_menu_command).into()
    } else if clsid == CLSID_EXPLORER_TAGS_PROP_HANDLER {
        ClassFactory::new(create_property_handler).into()
    } else {
        return CLASS_E_CLASSNOTAVAILABLE;
    };

    factory.query(&*riid, ppv)
}

#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    registration::register_server(get_hinstance())
}

#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    registration::unregister_server(get_hinstance())
}
