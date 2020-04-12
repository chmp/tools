//! Helpers to specify a the state of a directory tree in unit tests
#![allow(dead_code)]
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use tools_utils::{Error, Result};
use utime;

/// Specification of how the file tree should look like after copying
pub struct Spec {
    // NOTE: The tempdir drop function will delete the directory
    tempdir: TempDir,
    now: u64,
    expected_files: Vec<FileSpec>,
    expected_directories: Vec<PathBuf>,
}

/// Specification for individial files
///
/// They can have optional assertions on their content and modification date.
pub struct FileSpec {
    path: PathBuf,
    content: Option<String>,
    when: Option<u64>,
}

/// A helper trait to allow treating simple types as relative paths
pub trait RelativePathLike {
    fn to_path(self, root: impl AsRef<Path>) -> PathBuf;
}

impl RelativePathLike for &str {
    fn to_path(self, root: impl AsRef<Path>) -> PathBuf {
        root.as_ref().join(self)
    }
}

impl RelativePathLike for (&str, &str) {
    fn to_path(self, root: impl AsRef<Path>) -> PathBuf {
        root.as_ref().join(self.0).join(self.1)
    }
}

impl RelativePathLike for (&str, &str, &str) {
    fn to_path(self, root: impl AsRef<Path>) -> PathBuf {
        root.as_ref().join(self.0).join(self.1).join(self.2)
    }
}

impl RelativePathLike for (&str, &str, &str, &str) {
    fn to_path(self, root: impl AsRef<Path>) -> PathBuf {
        root.as_ref()
            .join(self.0)
            .join(self.1)
            .join(self.2)
            .join(self.3)
    }
}

impl Spec {
    pub fn new() -> Result<Self> {
        let result = Self {
            tempdir: tempfile::tempdir()
                .map_err(|e| format!("Could not create temporary directrory: {}", e))?,
            now: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| format!("Could not determine time since epoch: {}", e))?
                .as_secs(),
            expected_files: Vec::new(),
            expected_directories: Vec::new(),
        };
        Ok(result)
    }

    pub fn with_directory(self, path: impl RelativePathLike) -> Result<Self> {
        let path = path.to_path(self.tempdir.path());
        self.add_directory(path)?;
        Ok(self)
    }

    pub fn add_directory(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::create_dir_all(path)
            .map_err(|e| format!("Spec::with_dir: Cannot create directory : {}", e))?;
        Ok(())
    }

    pub fn path(&self, path: impl RelativePathLike) -> PathBuf {
        path.to_path(self.tempdir.path())
    }

    /// Create a file with given content and modification date
    ///
    /// Arguments:
    ///
    /// * `path`: the path of the file relative to the test root
    /// * `content`: if given, the content of the file, otherwise the create
    ///   file will be empty
    /// * `mtime`: if given the modification time of the file, otherwise the
    ///   start of the test
    ///
    pub fn with_file(
        self,
        path: impl RelativePathLike,
        content: Option<&str>,
        mtime: Option<u64>,
    ) -> Result<Self> {
        let path = path.to_path(self.tempdir.path());
        if let Some(parent) = path.parent() {
            self.add_directory(parent)?;
        }

        let content = content.map(|s| s.to_owned()).unwrap_or_default();
        let mtime = mtime.unwrap_or_default();

        let mut file = File::create(&path)
            .map_err(|e| format!("Spec::with_file: Cannot create file: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Spec::with_file: Cannot write content: {}", e))?;

        if mtime > 600 {
            return Err(Error::from("Cannot use times larger than 10 minutes"));
        }

        utime::set_file_times(&path, self.now + mtime - 600, self.now + mtime - 600).unwrap();

        Ok(self)
    }

    /// Add the expectation of a file to this spec
    ///
    /// Arguments:
    ///
    /// * `path`: the path of the file, relative to the test root
    /// * `content`: if given, the expected content of the file, if not given
    ///   it's only checked that file exists
    /// * `when`: if given, the expected modification time, relative to test
    ///   start.
    ///
    pub fn expect_file(
        mut self,
        path: impl RelativePathLike,
        content: Option<&str>,
        when: Option<u64>,
    ) -> Self {
        let path = path.to_path(self.tempdir.path());
        let content = content.map(|s| s.to_owned());

        let file_spec = FileSpec {
            path,
            content,
            when,
        };
        self.expected_files.push(file_spec);
        self
    }

    pub fn expect_directory(mut self, path: impl RelativePathLike) -> Self {
        let path = path.to_path(self.tempdir.path());
        self.expected_directories.push(path);
        self
    }

    pub fn assert(&self) -> Result<()> {
        for expected_directory in &self.expected_directories {
            assert!(
                expected_directory.exists(),
                "Expected path {:?} did not exist",
                expected_directory,
            );
            assert!(
                expected_directory.is_dir(),
                "Expected path {:?} is not a directory",
                expected_directory,
            );
        }

        for expected_file in &self.expected_files {
            assert!(
                expected_file.path.exists(),
                "Expected path {:?} did not exist",
                expected_file.path,
            );

            if let Some(expected) = &expected_file.content {
                let actual = read_file(&expected_file.path)?;
                assert_eq!(&actual, expected);
            }

            // TODO: compare mtimes times if given
        }
        Ok(())
    }
}

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("read_file: could not read file: {}", e))?;
    Ok(contents)
}
