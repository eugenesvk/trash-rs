use crate::{
    canonicalize_paths,
    macos::{percent_encode, DeleteMethod, TrashContextExtMacos},
    tests::{get_unique_name, init_logging},
    TrashContext,
};
use serial_test::serial;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[serial]
fn test_delete_with_finder_with_info() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let mut path1 = PathBuf::from(get_unique_name());
    let mut path2 = PathBuf::from(get_unique_name());
    path1.set_extension(r#"a"b,"#);
    path2.set_extension(r#"x80=%80 slash=\ pc=% quote=" comma=,"#);
    File::create_new(&path1).unwrap();
    File::create_new(&path2).unwrap();
    let trashed_items = trash_ctx.delete_all_with_info(&[path1.clone(), path2.clone()]).unwrap().unwrap(); //Ok + Some trashed paths
    assert!(File::open(&path1).is_err()); // original files deleted
    assert!(File::open(&path2).is_err());
    for item in trashed_items {
        let trashed_path = item.id;
        assert!(!File::open(&trashed_path).is_err()); // returned trash items exist
        std::fs::remove_file(&trashed_path).unwrap(); // clean   up
        assert!(File::open(&trashed_path).is_err()); // cleaned up trash items
    }

    // test a single file (in case returned paths aren't an array anymore)
    let mut path3 = PathBuf::from(get_unique_name());
    path3.set_extension(r#"a"b,"#);
    File::create_new(&path3).unwrap();
    let item = trash_ctx.delete_with_info(&path3).unwrap().unwrap(); //Ok + Some trashed paths
    assert!(File::open(&path3).is_err()); // original files deleted
    let trashed_path = item.id;
    assert!(!File::open(&trashed_path).is_err()); // returned trash items exist
    std::fs::remove_file(&trashed_path).unwrap(); // clean   up
    assert!(File::open(&trashed_path).is_err()); // cleaned up trash items
}

#[test]
#[serial]
fn test_delete_binary_with_finder_with_info() {
    init_logging();
    let (_cleanup, tmp) = create_hfs_volume().unwrap();
    let parent_fs_supports_binary = tmp.path();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let mut path1 = parent_fs_supports_binary.join(get_unique_name());
    let mut path2 = parent_fs_supports_binary.join(get_unique_name());
    path1.set_extension(OsStr::from_bytes(b"\x80a\"b")); // \x80 = lone continuation byte (128) (invalid utf8)
    path2.set_extension(OsStr::from_bytes(b"\x80=%80 slash=\\ pc=% quote=\" comma=,"));
    File::create_new(&path1).unwrap();
    File::create_new(&path2).unwrap();
    assert!(&path1.exists());
    assert!(&path2.exists());
    let trashed_items = trash_ctx.delete_all_with_info(&[path1.clone(), path2.clone()]).unwrap().unwrap(); //Ok + Some trashed paths
    assert!(File::open(&path1).is_err()); // original files deleted
    assert!(File::open(&path2).is_err());
    for item in trashed_items {
        let trashed_path = item.id;
        assert!(!File::open(&trashed_path).is_err()); // returned trash items exist
        std::fs::remove_file(&trashed_path).unwrap(); // clean   up
        assert!(File::open(&trashed_path).is_err()); // cleaned up trash items
    }
}

#[test]
#[serial]
fn test_delete_with_finder_quoted_paths() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let mut path1 = PathBuf::from(get_unique_name());
    let mut path2 = PathBuf::from(get_unique_name());
    path1.set_extension(r#"a"b,"#);
    path2.set_extension(r#"x80=%80 slash=\ pc=% quote=" comma=,"#);
    File::create_new(&path1).unwrap();
    File::create_new(&path2).unwrap();
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
    assert!(!path1.exists());
    assert!(!path2.exists());
}

#[test]
#[serial]
fn test_delete_with_ns_file_manager() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

    let path = get_unique_name();
    File::create_new(&path).unwrap();
    trash_ctx.delete(&path).unwrap();
    assert!(File::open(&path).is_err());
}

#[test]
#[serial]
fn test_delete_with_finder() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let path = PathBuf::from(get_unique_name());
    File::create_new(&path).unwrap();
    assert!(path.exists());
    trash_ctx.delete(&path).unwrap();
    assert!(!path.exists());
}

#[test]
#[serial]
fn test_delete_binary_path_with_ns_file_manager_with_info() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

    let mut path = PathBuf::from(get_unique_name());
    let paths = canonicalize_paths(&[path]).unwrap(); // need full path to get parent
    assert!(!paths.is_empty());
    path = paths[0].clone();
    let name = path.file_name().unwrap();
    let original_parent = path.parent().unwrap();

    File::create_new(&path).unwrap();
    let trash_item = trash_ctx.delete_with_info(&path).unwrap().unwrap();
    assert!(File::open(&path).is_err());
    assert_eq!(name, trash_item.name);
    assert_eq!(original_parent, trash_item.original_parent);
    // TrashItem's date deleted not tested since we can't guarantee the date we calculate here will match the date calculated @ delete
    // TrashItem's path@trash not tested since we can't guarantee the ours will be identical to the one FileManager decides to use (names change if they're dupe, also trash path is a bit tricky to get right as it changes depending on the user/admin)
}

#[test]
#[serial]
fn test_delete_binary_path_with_ns_file_manager() {
    let (_cleanup, tmp) = create_hfs_volume().unwrap();
    let parent_fs_supports_binary = tmp.path();

    init_logging();
    for method in [DeleteMethod::NsFileManager, DeleteMethod::Finder] {
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(method);

        let mut path_invalid = parent_fs_supports_binary.join(get_unique_name());
        path_invalid.set_extension(OsStr::from_bytes(b"\x80\"\\")); //...trash-test-111-0.\x80 (not push to avoid fail unexisting dir)

        File::create_new(&path_invalid).unwrap();

        assert!(path_invalid.exists());
        trash_ctx.delete(&path_invalid).unwrap();
        assert!(!path_invalid.exists());
    }
}

#[test]
fn test_path_byte() {
    let invalid_utf8 = b"\x80"; // lone continuation byte (128) (invalid utf8)
    let percent_encoded = "%80"; // valid macOS path in a %-escaped encoding

    let mut expected_path = PathBuf::from(get_unique_name());
    let mut path_with_invalid_utf8 = expected_path.clone();

    path_with_invalid_utf8.push(OsStr::from_bytes(invalid_utf8)); //      trash-test-111-0/\x80
    expected_path.push(percent_encoded); //                    trash-test-111-0/%80

    let actual = percent_encode(&path_with_invalid_utf8.as_os_str().as_encoded_bytes()); // trash-test-111-0/%80
    assert_eq!(std::path::Path::new(actual.as_ref()), expected_path);
}

fn create_hfs_volume() -> std::io::Result<(impl Drop, tempfile::TempDir)> {
    let tmp = tempfile::tempdir()?;
    let dmg_file = tmp.path().join("fs.dmg");
    let cleanup = {
        // Create dmg file
        Command::new("hdiutil").args(["create", "-size", "1m", "-fs", "HFS+"]).arg(&dmg_file).status()?;

        // Mount dmg file into temporary location
        Command::new("hdiutil").args(["attach", "-nobrowse", "-mountpoint"]).arg(tmp.path()).arg(&dmg_file).status()?;

        // Ensure that the mount point is always cleaned up
        defer::defer({
            let mount_point = tmp.path().to_owned();
            move || {
                Command::new("hdiutil")
                    .arg("detach")
                    .arg(&mount_point)
                    .status()
                    .expect("detach temporary test dmg filesystem successfully");
            }
        })
    };
    Ok((cleanup, tmp))
}
