use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Resolution {
    Inside(PathBuf),
    Outside,
    Unresolved,
}

/// Canonicalize the unmodified path: `link/..` must be left for the kernel.
pub fn resolve(root: &Path, path: &Path) -> io::Result<Resolution> {
    // Resolve the boundary at this operation's entry.  Comparing a resolved
    // path to the caller's spelling of the root is wrong for aliases such as
    // `/var` -> `/private/var` and Windows extended-length paths.
    let root = fs::canonicalize(root)?;
    match fs::canonicalize(path) {
        Ok(actual) if actual.starts_with(&root) => Ok(Resolution::Inside(actual)),
        Ok(_) => Ok(Resolution::Outside),
        Err(error) if unresolved(&error) => Ok(Resolution::Unresolved),
        Err(error) => Err(error),
    }
}
fn unresolved(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    ) || cfg!(unix) && error.raw_os_error() == Some(libc::ELOOP)
}

#[derive(Debug)]
pub enum ReadError {
    InputChanged,
    Unsafe,
    #[cfg_attr(unix, allow(dead_code))]
    Unsupported,
    Io(io::Error),
}
impl ReadError {
    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        match self {
            Self::Io(error) => Some(error.kind()),
            _ => None,
        }
    }
}
impl From<io::Error> for ReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity {
    len: u64,
    modified: Option<std::time::SystemTime>,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(unix)]
    mtime: i64,
    #[cfg(unix)]
    mtime_nsec: i64,
    #[cfg(unix)]
    ctime: i64,
    #[cfg(unix)]
    ctime_nsec: i64,
    #[cfg(windows)]
    volume_serial: u64,
    #[cfg(windows)]
    file_id: [u8; 16],
    #[cfg(windows)]
    change_time: i64,
}
fn identity(file: &File, metadata: &fs::Metadata) -> Result<Identity, ReadError> {
    #[cfg(unix)]
    {
        let _ = file;
        Ok(unix_identity(metadata))
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_BASIC_INFO, FILE_ID_INFO, FileBasicInfo, FileIdInfo, GetFileInformationByHandleEx,
        };

        let mut id = FILE_ID_INFO::default();
        // SAFETY: both pointers name initialized buffers of the exact Win32
        // structure sizes, and `file` owns a valid handle for this call.
        let id_ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileIdInfo,
                std::ptr::addr_of_mut!(id).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        };
        if id_ok == 0 {
            return Err(windows_identity_error());
        }
        let mut basic = FILE_BASIC_INFO::default();
        let basic_ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileBasicInfo,
                std::ptr::addr_of_mut!(basic).cast(),
                std::mem::size_of::<FILE_BASIC_INFO>() as u32,
            )
        };
        if basic_ok == 0 {
            return Err(windows_identity_error());
        }
        Ok(Identity {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            volume_serial: id.VolumeSerialNumber,
            file_id: id.FileId.Identifier,
            change_time: basic.ChangeTime,
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (file, metadata);
        Err(ReadError::Unsupported)
    }
}
#[cfg(unix)]
fn unix_identity(metadata: &fs::Metadata) -> Identity {
    use std::os::unix::fs::MetadataExt;
    Identity {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        dev: metadata.dev(),
        ino: metadata.ino(),
        mtime: metadata.mtime(),
        mtime_nsec: metadata.mtime_nsec(),
        ctime: metadata.ctime(),
        ctime_nsec: metadata.ctime_nsec(),
    }
}
#[cfg(windows)]
fn windows_identity_error() -> ReadError {
    use windows_sys::Win32::Foundation::{
        ERROR_CALL_NOT_IMPLEMENTED, ERROR_INVALID_FUNCTION, ERROR_INVALID_PARAMETER,
        ERROR_NOT_SUPPORTED,
    };

    let error = io::Error::last_os_error();
    match error.raw_os_error() {
        Some(code)
            if code == ERROR_INVALID_FUNCTION as i32
                || code == ERROR_NOT_SUPPORTED as i32
                || code == ERROR_INVALID_PARAMETER as i32
                || code == ERROR_CALL_NOT_IMPLEMENTED as i32 =>
        {
            ReadError::Unsupported
        }
        _ => ReadError::Io(error),
    }
}

/// Read a contained regular file with identity/path rechecks. Unix uses
/// O_NONBLOCK so a FIFO substitution cannot block. This is change detection,
/// not an atomic filesystem sandbox.
pub fn read_safe(root: &Path, path: &Path) -> Result<String, ReadError> {
    // Keep this resolved boundary for the whole operation.  In particular, do
    // not resolve the root again after a hook/open: a replacement at its
    // original spelling must not make a new directory an accepted boundary.
    let root = fs::canonicalize(root)?;
    let before_path = fs::canonicalize(path)?;
    #[cfg(test)]
    if forced_unsupported(&before_path) {
        return Err(ReadError::Unsupported);
    }
    if !before_path.starts_with(&root) {
        return Err(ReadError::Unsafe);
    }
    let before_metadata = fs::metadata(&before_path)?;
    if !before_metadata.is_file() {
        return Err(ReadError::Unsafe);
    }
    let guard = open_read(&before_path).map_err(after_open_error)?;
    let guard_before = guard.metadata()?;
    if !guard_before.is_file() {
        return Err(ReadError::InputChanged);
    }
    if before_metadata.len() != guard_before.len()
        || before_metadata.modified().ok() != guard_before.modified().ok()
    {
        return Err(ReadError::InputChanged);
    }
    let before = identity(&guard, &guard_before)?;
    #[cfg(unix)]
    if unix_identity(&before_metadata) != before {
        return Err(ReadError::InputChanged);
    }
    test_hook(HookPoint::BeforeOpen);
    let mut file = open_read(path).map_err(after_open_error)?;
    test_hook(HookPoint::AfterOpen);
    let read_before = file.metadata()?;
    if !read_before.is_file() || identity(&file, &read_before)? != before {
        return Err(ReadError::InputChanged);
    }
    let mut source = String::new();
    file.read_to_string(&mut source)?;
    test_hook(HookPoint::AfterRead);
    let read_after = file.metadata()?;
    let guard_after = guard.metadata()?;
    let after_path = fs::canonicalize(path).map_err(|_| ReadError::InputChanged)?;
    if after_path != before_path || !after_path.starts_with(&root) {
        return Err(ReadError::InputChanged);
    }
    let after_metadata = fs::metadata(&after_path).map_err(|_| ReadError::InputChanged)?;
    if !after_metadata.is_file() {
        return Err(ReadError::InputChanged);
    }
    let path_file = open_read(path).map_err(|_| ReadError::InputChanged)?;
    let path_handle = path_file.metadata().map_err(|_| ReadError::InputChanged)?;
    if !read_after.is_file()
        || identity(&file, &read_after)? != before
        || !guard_after.is_file()
        || identity(&guard, &guard_after)? != before
        || !path_handle.is_file()
        || identity(&path_file, &path_handle)? != before
    {
        return Err(ReadError::InputChanged);
    }
    Ok(source)
}
fn open_read(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    options.open(path)
}
fn after_open_error(error: io::Error) -> ReadError {
    if matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    ) {
        ReadError::InputChanged
    } else {
        ReadError::Io(error)
    }
}
pub fn regular(path: &Path) -> io::Result<bool> {
    Ok(fs::metadata(path)?.is_file())
}
pub fn directory(path: &Path) -> io::Result<bool> {
    Ok(fs::metadata(path)?.is_dir())
}

#[derive(Clone, Copy)]
enum HookPoint {
    BeforeOpen,
    AfterOpen,
    AfterRead,
}
#[cfg(test)]
type TestHook = Box<dyn Fn(HookPoint)>;
#[cfg(test)]
std::thread_local! {
    static TEST_HOOK: std::cell::RefCell<Option<TestHook>> = const { std::cell::RefCell::new(None) };
    static TEST_FORCE_UNSUPPORTED: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
fn forced_unsupported(path: &Path) -> bool {
    TEST_FORCE_UNSUPPORTED.with(|forced| forced.borrow().as_deref() == Some(path))
}
#[cfg(test)]
pub(crate) fn set_test_force_unsupported(path: Option<PathBuf>) {
    TEST_FORCE_UNSUPPORTED
        .with(|forced| *forced.borrow_mut() = path.and_then(|path| fs::canonicalize(path).ok()));
}
#[cfg(test)]
fn test_hook(point: HookPoint) {
    TEST_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow().as_ref() {
            hook(point);
        }
    });
}
#[cfg(not(test))]
fn test_hook(_: HookPoint) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use tempfile::TempDir;

    static TEST_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn unpoisoned_lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    struct HookGuard;
    impl Drop for HookGuard {
        fn drop(&mut self) {
            TEST_HOOK.with(|hook| *hook.borrow_mut() = None);
        }
    }
    fn with_hook(f: impl Fn(HookPoint) + 'static) -> HookGuard {
        TEST_HOOK.with(|hook| *hook.borrow_mut() = Some(Box::new(f)));
        HookGuard
    }
    #[cfg(unix)]
    fn complete_within<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = work();
            let _ = sender.send(result);
        });
        receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("FIFO read exceeded the watchdog deadline")
    }
    #[test]
    fn detects_replaced_path_and_changed_contents() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file = root.join("x");
        fs::write(&file, "one").unwrap();
        let swapped = Arc::new(AtomicBool::new(false));
        let copy = file.clone();
        let flag = swapped.clone();
        let hook = with_hook(move |point| {
            if matches!(point, HookPoint::BeforeOpen) && !flag.swap(true, Ordering::SeqCst) {
                fs::remove_file(&copy).unwrap();
                fs::write(&copy, "two longer").unwrap();
            }
        });
        assert!(matches!(
            read_safe(root, &file),
            Err(ReadError::InputChanged)
        ));
        drop(hook);
        let copy = file.clone();
        let _hook = with_hook(move |point| {
            if matches!(point, HookPoint::AfterRead) {
                fs::write(&copy, "changed content").unwrap();
            }
        });
        assert!(matches!(
            read_safe(root, &file),
            Err(ReadError::InputChanged)
        ));
    }
    #[test]
    fn reads_ordinary_file() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("x");
        fs::write(&file, "ordinary").unwrap();
        assert_eq!(read_safe(temp.path(), &file).unwrap(), "ordinary");
    }
    #[cfg(unix)]
    #[test]
    fn reads_through_root_alias() {
        use std::os::unix::fs::symlink;
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let file = root.join("x");
        fs::write(&file, "ordinary").unwrap();
        let alias = temp.path().join("alias");
        symlink(&root, &alias).unwrap();
        assert_eq!(read_safe(&alias, &file).unwrap(), "ordinary");
    }
    fn restore_mtime(path: &Path, metadata: &fs::Metadata) {
        let times = fs::FileTimes::new().set_modified(metadata.modified().unwrap());
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap()
            .set_times(times)
            .unwrap();
    }
    #[test]
    fn detects_before_open_replacement_with_same_length_and_restored_mtime() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, "one").unwrap();
        let original = fs::metadata(&file).unwrap();
        let copy = file.clone();
        let _hook = with_hook(move |point| {
            if matches!(point, HookPoint::BeforeOpen) {
                fs::remove_file(&copy).unwrap();
                fs::write(&copy, "two").unwrap();
                restore_mtime(&copy, &original);
                let current = fs::metadata(&copy).unwrap();
                assert_eq!(current.len(), original.len());
                assert_eq!(current.modified().unwrap(), original.modified().unwrap());
            }
        });
        assert!(matches!(
            read_safe(temp.path(), &file),
            Err(ReadError::InputChanged)
        ));
    }
    #[test]
    fn detects_after_read_replacement_with_same_length_and_restored_mtime() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, "one").unwrap();
        let original = fs::metadata(&file).unwrap();
        let copy = file.clone();
        let _hook = with_hook(move |point| {
            if matches!(point, HookPoint::AfterRead) {
                fs::remove_file(&copy).unwrap();
                fs::write(&copy, "two").unwrap();
                restore_mtime(&copy, &original);
                let current = fs::metadata(&copy).unwrap();
                assert_eq!(current.len(), original.len());
                assert_eq!(current.modified().unwrap(), original.modified().unwrap());
            }
        });
        assert!(matches!(
            read_safe(temp.path(), &file),
            Err(ReadError::InputChanged)
        ));
    }
    #[test]
    fn detects_after_read_in_place_change_with_same_length_and_restored_mtime() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, "one").unwrap();
        let original = fs::metadata(&file).unwrap();
        #[cfg(windows)]
        let before_identity = {
            let handle = File::open(&file).unwrap();
            identity(&handle, &original).unwrap()
        };
        let copy = file.clone();
        let _hook = with_hook(move |point| {
            if matches!(point, HookPoint::AfterRead) {
                // Do not replace the directory entry: this must exercise
                // Unix ctime / Windows ChangeTime on the same file identity.
                fs::write(&copy, "two").unwrap();
                restore_mtime(&copy, &original);
                let current = fs::metadata(&copy).unwrap();
                assert_eq!(current.len(), original.len());
                assert_eq!(current.modified().unwrap(), original.modified().unwrap());
                #[cfg(windows)]
                {
                    let handle = File::open(&copy).unwrap();
                    let restored_identity = identity(&handle, &current).unwrap();
                    eprintln!(
                        "in-place safe-read diagnostic: before_identity={before_identity:?}; \
                         restored_identity={restored_identity:?}; before_len={}; restored_len={}; \
                         before_modified={:?}; restored_modified={:?}",
                        original.len(),
                        current.len(),
                        original.modified().ok(),
                        current.modified().ok(),
                    );
                }
            }
        });
        let result = read_safe(temp.path(), &file);
        assert!(
            matches!(result, Err(ReadError::InputChanged)),
            "expected InputChanged after same-length in-place rewrite with restored mtime; actual result: {result:?}"
        );
    }
    #[cfg(windows)]
    #[test]
    fn windows_file_id_distinguishes_same_length_same_mtime_files() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::write(&first, "one").unwrap();
        let first_metadata = fs::metadata(&first).unwrap();
        fs::write(&second, "two").unwrap();
        restore_mtime(&second, &first_metadata);
        let second_metadata = fs::metadata(&second).unwrap();
        assert_eq!(first_metadata.len(), second_metadata.len());
        assert_eq!(
            first_metadata.modified().unwrap(),
            second_metadata.modified().unwrap()
        );
        let first_file = File::open(&first).unwrap();
        let second_file = File::open(&second).unwrap();
        let first_identity = identity(&first_file, &first_file.metadata().unwrap()).unwrap();
        let second_identity = identity(&second_file, &second_file.metadata().unwrap()).unwrap();
        assert_ne!(
            (first_identity.volume_serial, first_identity.file_id),
            (second_identity.volume_serial, second_identity.file_id)
        );
        let first_again = File::open(&first).unwrap();
        assert_eq!(
            identity(&first_again, &first_again.metadata().unwrap()).unwrap(),
            first_identity
        );
    }
    #[cfg(unix)]
    #[test]
    fn detects_link_target_change() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        use std::os::unix::fs::symlink;
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::write(root.join("one"), "one").unwrap();
        fs::write(root.join("two"), "two longer").unwrap();
        let link = root.join("link");
        symlink("one", &link).unwrap();
        let copy = link.clone();
        let _hook = with_hook(move |point| {
            if matches!(point, HookPoint::BeforeOpen) {
                fs::remove_file(&copy).unwrap();
                symlink("two", &copy).unwrap();
            }
        });
        assert!(matches!(
            read_safe(root, &link),
            Err(ReadError::InputChanged)
        ));
    }
    #[cfg(unix)]
    #[test]
    fn rejects_root_rebound_during_read() {
        use std::os::unix::fs::symlink;
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let old = temp.path().join("old");
        let new = temp.path().join("new");
        fs::create_dir(&old).unwrap();
        fs::create_dir(&new).unwrap();
        fs::write(old.join("x"), "old").unwrap();
        fs::write(new.join("x"), "new content").unwrap();
        let root = temp.path().join("root");
        symlink(&old, &root).unwrap();
        let path = root.join("x");
        let root_copy = root.clone();
        let new_copy = new.clone();
        let _hook = with_hook(move |point| {
            if matches!(point, HookPoint::BeforeOpen) {
                fs::remove_file(&root_copy).unwrap();
                symlink(&new_copy, &root_copy).unwrap();
            }
        });
        assert!(matches!(
            read_safe(&root, &path),
            Err(ReadError::InputChanged)
        ));
    }
    #[cfg(unix)]
    #[test]
    fn fifo_is_never_read_blocking() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        use std::os::unix::fs::FileTypeExt;
        let temp = TempDir::new().unwrap();
        let fifo = temp.path().join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(fs::symlink_metadata(&fifo).unwrap().file_type().is_fifo());
        let root = temp.path().to_path_buf();
        assert!(matches!(
            complete_within(move || read_safe(&root, &fifo)),
            Err(ReadError::Unsafe)
        ));
    }
    #[cfg(unix)]
    #[test]
    fn fifo_substitution_is_input_changed_and_nonblocking() {
        let _serial = unpoisoned_lock(&TEST_SERIAL);
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file");
        fs::write(&file, "ordinary").unwrap();
        let root = temp.path().to_path_buf();
        let opened = Arc::new(AtomicBool::new(false));
        let opened_copy = opened.clone();
        assert!(matches!(
            complete_within(move || {
                let copy = file.clone();
                let _hook = with_hook(move |point| {
                    if matches!(point, HookPoint::BeforeOpen) {
                        fs::remove_file(&copy).unwrap();
                        assert!(
                            std::process::Command::new("mkfifo")
                                .arg(&copy)
                                .status()
                                .unwrap()
                                .success()
                        );
                    }
                    if matches!(point, HookPoint::AfterOpen) {
                        opened_copy.store(true, Ordering::SeqCst);
                    }
                });
                read_safe(&root, &file)
            }),
            Err(ReadError::InputChanged)
        ));
        assert!(opened.load(Ordering::SeqCst));
    }
}
