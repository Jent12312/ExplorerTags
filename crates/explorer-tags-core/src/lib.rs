pub mod desktop_ini;
pub mod file_id;
pub mod guids;
pub mod icon_tint;
pub mod registry;
pub mod shell_notify;
pub mod tags_store;

pub use desktop_ini::{
    apply_folder_settings, get_folder_state, read_desktop_ini, remove_folder_color,
    remove_folder_tag, reset_folder, write_desktop_ini, FolderState, IniDocument,
};
pub use file_id::get_folder_file_id;
pub use guids::*;
pub use icon_tint::{get_or_create_tinted_icon, tint_ico_bytes};
pub use registry::{FolderRegistry, TaggedFolderRecord};
pub use shell_notify::notify_shell_folder_updated;
pub use tags_store::{TagsConfig, TagsStore};
