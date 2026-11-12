use explorer_tags_core::registry::{FolderRegistry, TaggedFolderRecord};
use tempfile::tempdir;

#[test]
fn test_registry_add_find_and_rename() {
    let dir = tempdir().expect("tempdir");
    let reg_path = dir.path().join("registry.json");

    let mut reg = FolderRegistry::load_from_path(reg_path.clone());
    assert_eq!(reg.all().len(), 0);

    let sample_path = dir.path().join("MyProject");
    let rec = TaggedFolderRecord {
        path: sample_path.clone(),
        tag: Some("⭐ Важно".to_string()),
        icon_path: None,
        color_hex: Some("FF5722".to_string()),
        volume_serial: Some(12345678),
        file_id: Some("0102030405060708090a0b0c0d0e0f10".to_string()),
        updated_at: 1000,
    };

    reg.upsert(rec);
    assert_eq!(reg.all().len(), 1);

    // Save and reload
    reg.save().expect("Save registry");
    let mut reg2 = FolderRegistry::load_from_path(reg_path);
    assert_eq!(reg2.all().len(), 1);

    // Find by path
    let found = reg2.find_by_path(&sample_path).expect("Should find by path");
    assert_eq!(found.tag.as_deref(), Some("⭐ Важно"));

    // Find by file ID
    let found_id = reg2.find_by_file_id(12345678, "0102030405060708090a0b0c0d0e0f10")
        .expect("Should find by file id");
    assert_eq!(found_id.path, sample_path);

    // Simulate folder rename
    let renamed_path = dir.path().join("MyProjectRenamed");
    let updated = reg2.update_path_by_file_id(12345678, "0102030405060708090a0b0c0d0e0f10", renamed_path.clone());
    assert!(updated, "Path should be updated by file id");

    let found_renamed = reg2.find_by_path(&renamed_path).expect("Should find by renamed path");
    assert_eq!(found_renamed.tag.as_deref(), Some("⭐ Важно"));
}

#[test]
fn test_get_folder_file_id_real_dir() {
    let dir = tempdir().expect("tempdir");
    let file_id_res = explorer_tags_core::file_id::get_folder_file_id(dir.path());
    // Note: on NTFS volumes this succeeds and returns (volume, hex_id). On FAT/exFAT it may not support FileIdInfo.
    if let Some((vol, hex_id)) = file_id_res {
        assert!(vol > 0);
        assert_eq!(hex_id.len(), 32); // 16 bytes = 32 hex chars
    }
}
