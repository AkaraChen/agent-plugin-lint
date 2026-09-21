use std::fs::{self, OpenOptions};
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
}
fn identity(metadata: &fs::Metadata) -> Identity {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Identity {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            dev: metadata.dev(),
            ino: metadata.ino(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        }
    }
    #[cfg(not(unix))]
    {
        Identity {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

/// Read a contained regular file with identity/path rechecks. Unix uses
/// O_NONBLOCK so a FIFO substitution cannot block. This is change detection,
/// not an atomic filesystem sandbox; non-Unix identity is necessarily weaker.
pub fn read_safe(root: &Path, path: &Path) -> Result<String, ReadError> {
    // Keep this resolved boundary for the whole operation.  In particular, do
    // not resolve the root again after a hook/open: a replacement at its
    // original spelling must not make a new directory an accepted boundary.
    let root = fs::canonicalize(root)?;
    let before_path = fs::canonicalize(path)?;
    if !before_path.starts_with(&root) {
        return Err(ReadError::Unsafe);
    }
    let before_metadata = fs::metadata(&before_path)?;
    if !before_metadata.is_file() {
        return Err(ReadError::Unsafe);
    }
    let before = identity(&before_metadata);
    test_hook(HookPoint::BeforeOpen);
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path).map_err(after_open_error)?;
    let handle_before = file.metadata()?;
    if !handle_before.is_file() || identity(&handle_before) != before {
        return Err(ReadError::InputChanged);
    }
    let mut source = String::new();
    file.read_to_string(&mut source)?;
    test_hook(HookPoint::AfterRead);
    let handle_after = file.metadata()?;
    let after_path = fs::canonicalize(path).map_err(|_| ReadError::InputChanged)?;
    if after_path != before_path || !after_path.starts_with(&root) {
        return Err(ReadError::InputChanged);
    }
    let after_metadata = fs::metadata(&after_path).map_err(|_| ReadError::InputChanged)?;
    if !handle_after.is_file()
        || identity(&handle_after) != before
        || identity(&after_metadata) != before
    {
        return Err(ReadError::InputChanged);
    }
    Ok(source)
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
    AfterRead,
}
#[cfg(test)]
type TestHook = Box<dyn Fn(HookPoint) + Send + Sync>;
#[cfg(test)]
static TEST_HOOK: std::sync::Mutex<Option<TestHook>> = std::sync::Mutex::new(None);
#[cfg(test)]
fn test_hook(point: HookPoint) {
    if let Some(hook) = TEST_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
    {
        hook(point);
    }
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
            *unpoisoned_lock(&TEST_HOOK) = None;
        }
    }
    fn with_hook(f: impl Fn(HookPoint) + Send + Sync + 'static) -> HookGuard {
        *unpoisoned_lock(&TEST_HOOK) = Some(Box::new(f));
        HookGuard
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
        assert!(matches!(
            read_safe(temp.path(), &fifo),
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
        });
        assert!(matches!(
            read_safe(temp.path(), &file),
            Err(ReadError::InputChanged)
        ));
    }
}
