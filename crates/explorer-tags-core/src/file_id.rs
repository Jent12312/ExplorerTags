use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetFileInformationByHandleEx, FileIdInfo, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_ID_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

/// Получает пару (VolumeSerialNumber, 128-битный File ID в виде HEX строки) для указанного пути папки/файла.
pub fn get_folder_file_id(path: &Path) -> Option<(u64, String)> {
    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    // FILE_FLAG_BACKUP_SEMANTICS необходим для открытия хэндла каталога
    // 0x80 = FILE_READ_ATTRIBUTES (без требования чтения содержимого)
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            0x80, // FILE_READ_ATTRIBUTES
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    };

    if handle.is_err() {
        return None;
    }

    let handle = handle.unwrap();
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut id_info = FILE_ID_INFO::default();
    let success = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            &mut id_info as *mut _ as _,
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };

    unsafe {
        let _ = CloseHandle(handle);
    }

    if success.is_err() {
        return None;
    }

    let hex_id: String = id_info
        .FileId
        .Identifier
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    Some((id_info.VolumeSerialNumber, hex_id))
}

use std::os::windows::ffi::OsStrExt;
