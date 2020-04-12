//! Helpers to run backups
use glob::Pattern;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use tools_utils::{Error, Result};
use walkdir::WalkDir;

/// Run a full backup
pub fn run_backup(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    reference: Option<impl AsRef<Path>>,
    ignore_spec: &impl IgnoreSpec,
) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();
    let reference = reference.as_ref().map(|p| p.as_ref());

    let mut walker = WalkDir::new(source)
        .min_depth(1)
        .contents_first(false)
        .into_iter();

    loop {
        let item = match walker.next() {
            None => break,
            Some(Err(e)) => {
                return Err(Error::from(format!(
                    "run_backup: Invalid directory entry: {}",
                    e
                )))
            }
            Some(Ok(entry)) => entry,
        };

        let item = item.path();
        if ignore_spec.is_ignored(&item)? {
            println!("skip {:?}", item);
            if item.is_dir() {
                walker.skip_current_dir();
            }
            continue;
        }
        let rel_item = item
            .strip_prefix(source)
            .map_err(|e| format!("Cannot determine relative path: {}", e))?;
        let target_item = target.join(&rel_item);
        let reference_item = reference.map(|p| p.join(&rel_item));

        backup_item(item, target_item, reference_item)?;
    }
    Ok(())
}

pub trait IgnoreSpec {
    fn is_ignored(&self, path: &Path) -> Result<bool>;
}

/// Specification of files to ignore using glob patterns
///
pub struct GlobIgnoreSpec {
    root: PathBuf,
    patterns: Vec<Pattern>,
}

impl GlobIgnoreSpec {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_owned(),
            patterns: Vec::new(),
        }
    }

    pub fn from_file(root: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)
            .map_err(|e| format!("load_ignore_patterns: could not open file: {}", e))?;
        let reader = BufReader::new(file);
        let mut result = Self::new(root);
        for line in reader.lines() {
            let line =
                line.map_err(|e| format!("load_ignore_patterns: could not read line: {}", e))?;
            let pattern = Pattern::new(&line)
                .map_err(|e| format!("load_ignore_patterns: could not compile pattern: {}", e))?;
            result.patterns.push(pattern);
        }
        Ok(result)
    }
}

impl IgnoreSpec for GlobIgnoreSpec {
    fn is_ignored(&self, path: &Path) -> Result<bool> {
        let rel_item = path
            .strip_prefix(&self.root)
            .map_err(|e| format!("Cannot determine relative path: {}", e))?;
        let pattern_item = PathBuf::from("root").join(rel_item);
        let result = self.patterns.iter().any(|p| p.matches_path(&pattern_item));
        Ok(result)
    }
}

pub struct NoOpIgnoreSpec;

impl IgnoreSpec for NoOpIgnoreSpec {
    fn is_ignored(&self, _path: &Path) -> Result<bool> {
        Ok(false)
    }
}

/// Backup an item (file, directory, or symlink)
///
/// Arguments:
///
/// * `source`: the source path from which the file will be copied
/// * `target` the target path that will be created
/// * `reference`: a previous backup if it exists. Will be used to check whether
///   a hard-link can be used to deduplicate the files.
///
pub fn backup_item(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    reference: Option<impl AsRef<Path>>,
) -> Result<()> {
    let source = source.as_ref();
    let metadata = source
        .metadata()
        .map_err(|e| format!("backup_item: could not retrieve metadata: {}", e))?;
    let file_type = metadata.file_type();

    if file_type.is_dir() {
        backup_directory(target)?;
    } else if file_type.is_file() {
        backup_file(source, target, reference)?;
    } else if file_type.is_symlink() {
        backup_symlink(source, target)?;
    }
    Ok(())
}

/// Backup a 'normal' file
///
/// Arguments:
///
/// * `source`: the source path from which the file will be copied
/// * `target` the target path that will be created
/// * `reference`: a previous backup if it exists. Will be used to check whether
///   a hard-link can be used to deduplicate the files.
///
pub fn backup_file(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    reference: Option<impl AsRef<Path>>,
) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();
    let reference = reference.as_ref().map(|r| r.as_ref());

    if let Some(parent) = target.parent() {
        ensure_directory_exists(parent)?;
    }

    if !should_link(source, reference) {
        println!("COPY {:?}", target);
        fs::copy(source, target)
            .map_err(|e| format!("backup_file: could not copy file: {:?}", e))?;
    } else {
        let reference = reference.unwrap();
        println!("LINK {:?}", reference);
        std::fs::hard_link(reference, target)
            .map_err(|e| format!("backup_file: could not create link: {}", e))?;
    }
    Ok(())
}

fn should_link(source: impl AsRef<Path>, reference: Option<impl AsRef<Path>>) -> bool {
    let reference = match reference {
        None => return false,
        Some(reference) => reference,
    };

    let ref_mod = fs::metadata(reference).and_then(|meta| meta.modified());
    let cur_mod = fs::metadata(source).and_then(|meta| meta.modified());
    match (ref_mod, cur_mod) {
        (Ok(ref_mod), Ok(cur_mod)) => ref_mod >= cur_mod,
        _ => false,
    }
}

/// Backup a directory
pub fn backup_directory(target: impl AsRef<Path>) -> Result<()> {
    let target = target.as_ref();

    if !target.exists() {
        println!("DIR  {:?}", target);
        fs::create_dir_all(target)
            .map_err(|e| format!("backup_directory: Could not create directory: {}", e))?;
    } else if !target.is_dir() {
        return Err(Error::from(
            "backup_directory: existing target is not a directory",
        ));
    }
    Ok(())
}

/// Backup a symlink
pub fn backup_symlink(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<()> {
    let source = source.as_ref();
    let target = target.as_ref();

    let src_item = std::fs::read_link(source)
        .map_err(|e| format!("backup_symlink: cannot read link: {}", e))?;
    println!("SYM  {:?} -> {:?}", target, src_item);
    if let Some(parent) = target.parent() {
        ensure_directory_exists(parent)?;
    }

    let content = format!(
        "LINK {}",
        src_item
            .to_str()
            .ok_or_else(|| Error::from("backup_symblink: Cannot represent path as utf8: {}"))?
    );

    let mut f =
        File::create(target).map_err(|e| format!("backup_symlink: cannot create file: {}", e))?;
    f.write_all(content.as_bytes())
        .map_err(|e| format!("backup_symlink: cannot write file: {}", e))?;

    Ok(())
}

pub fn ensure_directory_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    fs::create_dir_all(path)
        .map_err(|e| format!("ensure_directory_exists: could not create directory: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::test_spec::Spec;
    use super::*;

    #[test]
    fn backup_file_example_no_reference() -> Result<()> {
        let spec = Spec::new()?
            .with_file(("source", "foo.txt"), Some("hello"), None)?
            .expect_file(("target", "foo.txt"), Some("hello"), None);

        run_backup(
            spec.path("source"),
            spec.path("target"),
            Option::<&Path>::None,
            &NoOpIgnoreSpec,
        )?;

        spec.assert()?;

        Ok(())
    }

    #[test]
    fn backup_file_example_with_reference() -> Result<()> {
        let spec = Spec::new()?
            .with_file(("source", "foo.txt"), Some("hello"), Some(0))?
            .with_file(("reference", "foo.txt"), Some("world"), Some(1))?
            .expect_file(("target", "foo.txt"), Some("world"), None);

        run_backup(
            spec.path("source"),
            spec.path("target"),
            Some(spec.path("reference")),
            &NoOpIgnoreSpec,
        )?;

        spec.assert()?;

        Ok(())
    }

    #[test]
    fn backup_directory_example() -> Result<()> {
        let spec = Spec::new()?
            .with_directory(("source", "a"))?
            .expect_directory(("target", "a"));
        run_backup(
            spec.path("source"),
            spec.path("target"),
            Option::<&Path>::None,
            &NoOpIgnoreSpec,
        )?;
        spec.assert()?;
        Ok(())
    }

    #[test]
    fn test_run_backup() -> Result<()> {
        let spec = Spec::new()?
            .with_file(("source", "foo"), None, Some(1))?
            .with_file(("source", "bar", "baz"), None, Some(2))?
            .with_directory("target")?
            .expect_file(("target", "foo"), None, None)
            .expect_file(("target", "bar", "baz"), None, None);

        run_backup(
            spec.path("source"),
            spec.path("target"),
            Option::<&Path>::None,
            &NoOpIgnoreSpec,
        )?;
        spec.assert()?;
        Ok(())
    }

    #[test]
    fn test_run_backup_prev() -> Result<()> {
        let spec = Spec::new()?
            .with_file(("source", "foo"), Some("curr"), Some(1))?
            .with_file(("source", "bar", "baz"), Some("curr"), Some(2))?
            .with_file(("source", "hello", "world"), Some("curr"), Some(2))?
            .with_file(("prev", "foo"), Some("prev"), Some(1))?
            .with_file(("prev", "hello", "world"), Some("prev"), Some(1))?
            .with_directory("target")?
            .expect_file(("target", "foo"), Some("prev"), None)
            .expect_file(("target", "bar", "baz"), Some("curr"), None)
            .expect_file(("target", "hello", "world"), Some("curr"), None);

        run_backup(
            spec.path("source"),
            spec.path("target"),
            Some(spec.path("prev")),
            &NoOpIgnoreSpec,
        )?;
        spec.assert()?;
        Ok(())
    }
}
