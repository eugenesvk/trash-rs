#![cfg_attr(not(debug_assertions),allow(non_snake_case,non_upper_case_globals,non_camel_case_types))]
#![cfg_attr(    debug_assertions ,allow(non_snake_case,non_upper_case_globals,non_camel_case_types,unused_imports,unused_mut,unused_variables,dead_code,unused_assignments,unused_macros))]
use objc2::runtime::AnyObject;
use objc2_foundation::{NSUTF8StringEncoding,NSShiftJISStringEncoding};
use crate::fmt;
use std::{ffi::OsString, ffi::CString, path::PathBuf, path::Path, process::Command};
use std::ptr::NonNull;
use std::os::unix::ffi::OsStrExt;
use objc2::ClassType;
use objc2::rc::{Retained, Retained as Ret,Allocated};

use log::trace;
use objc2_foundation::{NSFileManager, NSString, NSURL};

use crate::{into_unknown, Error, TrashContext};

#[derive(Copy, Clone, Debug)]
pub enum DeleteMethod {
    /// Use an `osascript`, asking the Finder application to delete the files.
    ///
    /// - Might ask the user to give additional permissions to the app
    /// - Produces the sound that Finder usually makes when deleting a file
    /// - Shows the "Put Back" option in the context menu, when using the Finder application
    ///
    /// This is the default.
    Finder,

    /// Use `trashItemAtURL` from the `NSFileManager` object to delete the files.
    ///
    /// - Somewhat faster than the `Finder` method
    /// - Does *not* require additional permissions
    /// - Does *not* produce the sound that Finder usually makes when deleting a file
    /// - Does *not* show the "Put Back" option on some systems (the file may be restored by for
    ///   example dragging out from the Trash folder). This is a macOS bug. Read more about it
    ///   at:
    ///   - <https://github.com/sindresorhus/macos-trash/issues/4>
    ///   - <https://github.com/ArturKovacs/trash-rs/issues/14>
    NsFileManager,

    /// Use Rust std library to delete the files, storing original paths as extended attributes.
    ///
    /// - Somewhat faster than the `Finder` method
    /// - Does *not* require additional permissions
    /// - Does *not* produce the sound that Finder usually makes when deleting a file
    /// - Does *not* show the "Put Back" option, BUT replaces it with a custom one.
    Direct,
}
impl fmt::Display for DeleteMethod {
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result {
        match self {
            DeleteMethod::Finder        => write!(f,"Finder"),
            DeleteMethod::NsFileManager => write!(f,"NsFileManager"),
            DeleteMethod::Direct        => write!(f,"Direct"),
        }
    }
}

impl DeleteMethod {
    /// Returns `DeleteMethod::Finder`
    pub const fn new() -> Self {
        DeleteMethod::Finder
    }
}
impl Default for DeleteMethod {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone, Default, Debug)]
pub struct PlatformTrashContext {
    delete_method: DeleteMethod,
}
impl PlatformTrashContext {
    pub const fn new() -> Self {
        Self { delete_method: DeleteMethod::new() }
    }
}
pub trait TrashContextExtMacos {
    fn set_delete_method(&mut self, method: DeleteMethod);
    fn delete_method(&self) -> DeleteMethod;
}
impl TrashContextExtMacos for TrashContext {
    fn set_delete_method(&mut self, method: DeleteMethod) {
        self.platform_specific.delete_method = method;
    }
    fn delete_method(&self) -> DeleteMethod {
        self.platform_specific.delete_method
    }
}
impl TrashContext {
    pub(crate) fn delete_all_canonicalized(&self, full_paths: Vec<PathBuf>) -> Result<(), Error> {
        let full_paths = full_paths.into_iter().map(to_string).collect::<Result<Vec<_>, _>>()?;
        match self.platform_specific.delete_method {
            DeleteMethod::Finder => delete_using_finder(full_paths),
            DeleteMethod::NsFileManager => delete_using_file_mgr(full_paths),
            DeleteMethod::Direct => delete_directly(full_paths),
        }
    }
}

fn delete_using_file_mgr(full_paths: Vec<String>) -> Result<(), Error> {
    trace!("Starting delete_using_file_mgr");
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    for path in full_paths {
        let string = NSString::from_str(&path);

        trace!("Starting fileURLWithPath");
        let url = unsafe { NSURL::fileURLWithPath(&string) };
        trace!("Finished fileURLWithPath");

        trace!("Calling trashItemAtURL");
        let res = unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, None) };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{path}', `trashItemAtURL` failed: {err}"),
            });
        }
    }
    Ok(())
}

pub fn delete_using_file_mgr_oss<P:AsRef<Path>>(full_paths: &[P]) -> Result<(), Error> {
    trace!("Starting delete_using_file_mgr");
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    for path in full_paths {
        // let p_ostr = path.as_ref().as_os_str(); // if p_ostr.is_empty() {} // should've been checked at the canonicalization phase
        // let p_cstr = CString::new(p_ostr.as_bytes()).expect("CString::new failed to create from given path");
        // let p_cstr_len = p_cstr.count_bytes();
        // let p_nstr = NonNull::new(p_cstr.into_raw()).expect("CString from path shouldn't have a null ref!");
        // let string:Retained<NSString> = unsafe {file_mgr.stringWithFileSystemRepresentation_length(p_nstr, p_cstr_len)};


        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
// ok with utf8 via cstrings
        // // let pre:&str = "/Volumes/Untitled/ピ";
        // let bja8 = [0xe3,0x83,0x94];
        // let ja8:&OsStr = OsStr::from_bytes(&bja8);
        // let mut p:PathBuf = PathBuf::new(); p.push("/Volumes/Untitled"); p.push(ja8); let pja8 = p;
        // let prenbja8 = pja8.as_os_str().as_bytes();
        // let cstring = CString::new(prenbja8).expect("CString::new failed to create from given path");
        // let cstring_len = cstring.count_bytes();
        // println!("pre={:?}\nc{}={:?}",&prenbja8,cstring_len,&cstring);
        // let nncstring = NonNull::new(cstring.into_raw()).expect("REASON");
        // // 3 OK
        // let string:Retained<NSString> = unsafe {file_mgr.stringWithFileSystemRepresentation_length(nncstring, cstring_len)};

// todo: still fails with pjis
        // let pre:&str = "/Volumes/Untitled/ピ";
        let bja8 = [0xe3,0x83,0x94]; //227 131 148
        let bjis = [     0x83,0x73]; //    131 115
        let jis:&OsStr = OsStr::from_bytes(&bjis);
        let mut p:PathBuf = PathBuf::new(); p.push("/Volumes/Untitled"); p.push(jis); let pjis = p;
        if pjis.is_file() {println!("pjis is a file! {:?}",pjis);}
        let prenbjis:&[u8] = pjis.as_os_str().as_bytes();
        let cstring = CString::new(prenbjis).expect("CString::new failed to create from given path");
        let cstring_len = cstring.count_bytes();
        println!("pre={:?}\nc{}={:?}",&prenbjis,cstring_len,&cstring); //c20="/Volumes/Untitled/\x83s"
        let nncstring = NonNull::new(cstring.into_raw()).expect("REASON");

        use objc2_foundation::NSCharacterSet;
        let valid_charset: Retained<NSCharacterSet> = unsafe { NSCharacterSet::URLPathAllowedCharacterSet() };
        // 3 ✗ FAILS with jis
          // let string:Retained<NSString> = unsafe {file_mgr.stringWithFileSystemRepresentation_length(nncstring, cstring_len)};
          // let string:Retained<NSString> = unsafe {NSString::stringWithCString_encoding(nncstring, NSShiftJISStringEncoding).expect("")};
            // ↑ this deletes the wrong file, not our jis one, but the utf8 one
          // let string:Retained<NSString> = unsafe {NSString::stringWithCString_length(nncstring, cstring_len)};
          // println!("nsstring={:?}",&string);


        // ✗ FAILS , can't use anyobj
          // #[allow(deprecated)]
          // let anyobj:Retained<AnyObject> = unsafe {NSString::stringWithCString(nncstring).expect("REASON")};
          // println!("nsstring stringWithCString (anyobj)={:?} class={:?}",&anyobj,anyobj.class()); //<__NSCFString: 0x600001b50700>
          // no method// let anyobj_percent:Option<Retained<NSString>> = unsafe {anyobj.stringByAddingPercentEncodingWithAllowedCharacters(&valid_charset)};


// try to use binary data to create URL
  use objc2_foundation::NSData;
  let nsdata   	:Ret<NSData  >	= NSData::with_bytes(&prenbjis)                                        	;
  let url_d    	:Ret<NSURL   >	= unsafe{NSURL::URLWithDataRepresentation_relativeToURL(&nsdata,None) }	;// /Volumes/Untitled/%C2%83s
  let url_path 	:Ret<NSString>	= unsafe{url_d.path()}.expect("p")                                     	;// /Volumes/Untitled/\u{83}s
  let string_pc	:Ret<NSString>	= unsafe{url_path.stringByAddingPercentEncodingWithAllowedCharacters(&valid_charset)}.expect("e"); // /Volumes/Untitled/%C2%83s
  let url_p    	:Ret<NSURL   >	= unsafe{NSURL::fileURLWithPath(&url_path ) }	;
  let url_s    	:Ret<NSURL   >	= unsafe{NSURL::fileURLWithPath(&string_pc) }	;
  let res_p    	              	= unsafe{file_mgr.trashItemAtURL_resultingItemURL_error(&url_p, None) };// file “\u{83}s” doesn’t exist
  let res_s    	              	= unsafe{file_mgr.trashItemAtURL_resultingItemURL_error(&url_s, None) };// file “%C2%83s” doesn’t exist
  // let res_d 	              	= unsafe{file_mgr.trashItemAtURL_resultingItemURL_error(&url_d, None) };// couldn’t be opened because the specified URL type isn't supported
let err_s = match res_s {Ok(())=>format!("ok"), Err(err)=>format!("{err}"),};
let err_p = match res_p {Ok(())=>format!("ok"), Err(err)=>format!("{err}"),};
// let err_d = match res_d {Ok(())=>format!("ok"), Err(err)=>format!("{err}"),};
println!("NSURL from nsdata URLWithDataRepresentation_relativeToURL:\nurl_d={url_d:?}\nurl_p={url_path:?}\nstr_%= {string_pc}\nurl_p={url_p:?}\nurl_s={url_s:?}\nerr_p={err_p}\nres_s={err_s}");
  let url = url_p;
// ✓ works with ja8
  // "ピ" "\x83s"
  // pja8 is a file! "/Volumes/Untitled/ピ"
  // pre=[47, 86, 111, 108, 117, 109, 101, 115, 47, 85, 110, 116, 105, 116, 108, 101, 100, 47, 227, 131, 148]
  // let bja8 = [0xe3,0x83,0x94]; //227 131 148
  // c21="/Volumes/Untitled/\xe3\x83\x94"
  // NSURL from nsdata URLWithDataRepresentation_relativeToURL: NSURL { __superclass: /Volumes/Untitled/%E3%83%94 }
  // path="/Volumes/Untitled/ピ"
  // str%="/Volumes/Untitled/%E3%83%94"
  // url=NSURL { __superclass: file:///Volumes/Untitled/%E3%83%92%E3%82%9A }
// ✗ fails with jis
  // "ピ" "\x83s"
  // pjis is a file! "/Volumes/Untitled/\x83s"
  // pre=[47, 86, 111, 108, 117, 109, 101, 115, 47, 85, 110, 116, 105, 116, 108, 101, 100, 47, 131, 115]
  // let bjis = [     0x83,0x73]; //    131 115
  // c20="/Volumes/Untitled/\x83s"
  // NSURL from nsdata URLWithDataRepresentation_relativeToURL:
  // url_d=NSURL { __superclass: /Volumes/Untitled/%C2%83s }
  // url_path=/Volumes/Untitled/s
  // str%=/Volumes/Untitled/%C2%83s
  // url_s=NSURL { __superclass: file:///Volumes/Untitled/%25C2%2583s }
  // url_p=NSURL { __superclass: file:///Volumes/Untitled/%C2%83s }
  // res_s=The file “%C2%83s” doesn’t exist.
  // res_p=The file “s” doesn’t exist.
  // called `Result::unwrap()` on an `Err` value: Unknown { description: "While deleting '\"/Volumes/Untitled/ピ\"', `trashItemAtURL` failed: The file “\u{83}s” doesn’t exist." }

  // path="/Volumes/Untitled/\u{83}s"

        // let new_s:Allocated<NSString> = NSString::alloc();
        // #[allow(deprecated)]
        // let string:Retained<NSString> = unsafe {NSString::initWithCString(new_s,nncstring).expect("REASON")};
        // println!("nsstring initWithCString from allocated={:?}",&string); // "/Volumes/Untitled/És"

        // let string_percent:Option<Retained<NSString>> = unsafe {string.stringByAddingPercentEncodingWithAllowedCharacters(&valid_charset)};
        // println!("nsstring stringByAddingPercentEncodingWithAllowedCharacters={:?}",&string); //"/Volumes/Untitled/És"
        // let string = string_percent.expect("rea");

        // ✗ crash due to macOS 14.0+?: invalid message send to +[NSURL URLWithString:encodingInvalidCharacters:]: method not found
          // let encoding_invalid_characters = true;
          // let url = unsafe { NSURL::URLWithString_encodingInvalidCharacters(&string, encoding_invalid_characters) }.expect("sadfsdf");

        trace!("Starting fileURLWithPath");
        // let url = unsafe { NSURL::fileURLWithPath(&string) };
        // println!("NSURL: {:?}",url);
        trace!("Finished fileURLWithPath");

        trace!("Calling trashItemAtURL");
        let res = unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, None) };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{:?}', `trashItemAtURL` failed: {err}",path.as_ref()),
            });
        }
    }
    Ok(())
}

// TODO replace
fn delete_directly(full_paths: Vec<String>) -> Result<(), Error> {
    trace!("Starting delete_using_file_mgr");
    let file_mgr = unsafe { NSFileManager::defaultManager() };
    for path in full_paths {
        let string = NSString::from_str(&path);

        trace!("Starting fileURLWithPath");
        let url = unsafe { NSURL::fileURLWithPath(&string) };
        trace!("Finished fileURLWithPath");

        trace!("Calling trashItemAtURL");
        let res = unsafe { file_mgr.trashItemAtURL_resultingItemURL_error(&url, None) };
        trace!("Finished trashItemAtURL");

        if let Err(err) = res {
            return Err(Error::Unknown {
                description: format!("While deleting '{path}', `trashItemAtURL` failed: {err}"),
            });
        }
    }
    Ok(())
}

fn delete_using_finder(full_paths: Vec<String>) -> Result<(), Error> {
    // AppleScript command to move files (or directories) to Trash looks like
    //   osascript -e 'tell application "Finder" to delete { POSIX file "file1", POSIX "file2" }'
    // The `-e` flag is used to execute only one line of AppleScript.
    let mut command = Command::new("osascript");
    let posix_files = full_paths.into_iter().map(|p| format!("POSIX file \"{p}\"")).collect::<Vec<String>>().join(", ");
    let script = format!("tell application \"Finder\" to delete {{ {posix_files} }}");

    let argv: Vec<OsString> = vec!["-e".into(), script.into()];
    command.args(argv);

    // Execute command
    let result = command.output().map_err(into_unknown)?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        match result.status.code() {
            None => {
                return Err(Error::Unknown {
                    description: format!("The AppleScript exited with error. stderr: {}", stderr),
                })
            }

            Some(code) => {
                return Err(Error::Os {
                    code,
                    description: format!("The AppleScript exited with error. stderr: {}", stderr),
                })
            }
        };
    }
    Ok(())
}

fn to_string<T: Into<OsString>>(str_in: T) -> Result<String, Error> {
    let os_string = str_in.into();
    let s = os_string.to_str();
    match s {
        Some(s) => Ok(s.to_owned()),
        None => Err(Error::ConvertOsString { original: os_string }),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        macos::{DeleteMethod, TrashContextExtMacos},
        tests::{get_unique_name, init_logging},
        TrashContext,
    };
    use serial_test::serial;
    use std::fs::File;

    #[test]
    #[serial]
    fn test_delete_with_ns_file_manager() {
        init_logging();
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

        let path = get_unique_name();
        File::create(&path).unwrap();
        trash_ctx.delete(&path).unwrap();
        assert!(File::open(&path).is_err());
    }
}
