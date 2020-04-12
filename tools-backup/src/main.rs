/// Helper to handle backups in windows
mod backup;
mod sanitize_path;
mod test_spec;

use clap::{App, Arg};
use std::path::{Path, PathBuf};
use tools_utils::{run_main, Result};

use backup::{GlobIgnoreSpec, IgnoreSpec, NoOpIgnoreSpec};

fn main() {
    run_main(main_impl);
}

fn main_impl() -> Result<i32> {
    let arguments = parse_args()?;

    println!("Run backup");
    println!("Source: {:?}", arguments.source);
    println!("Target: {:?}", arguments.target);
    if let Some(reference) = &arguments.reference {
        println!("With reference: {:?}", reference);
    } else {
        println!("Without reference");
    }

    let ignore_file = arguments.source.join("wbck-ignore.txt");
    let ignore_spec: Box<dyn IgnoreSpec> = if ignore_file.exists() {
        println!("Read ignore spec from {:?}", ignore_file);
        Box::new(GlobIgnoreSpec::from_file(&arguments.source, &ignore_file)?)
    } else {
        Box::new(NoOpIgnoreSpec)
    };
    // run the actual backup
    backup::run_backup(
        &arguments.source,
        &arguments.target,
        arguments.reference.as_ref(),
        &ignore_spec,
    )?;

    Ok(0)
}

// see: https://users.rust-lang.org/t/boxed-trait-object-doesnt-impl-trait/24729
impl IgnoreSpec for Box<dyn IgnoreSpec> {
    fn is_ignored(&self, path: &Path) -> Result<bool> {
        self.as_ref().is_ignored(path)
    }
}

fn parse_args() -> Result<Arguments> {
    let matches = App::new("tools-backup")
        .arg(Arg::with_name("reference").long("ref").takes_value(true))
        .arg(Arg::with_name("source").required(true))
        .arg(Arg::with_name("target").required(true))
        .get_matches();
    let reference = matches.value_of_os("reference").map(PathBuf::from);
    let source = matches
        .value_of_os("source")
        .ok_or_else(|| String::from("Missing argument source"))?
        .into();
    let target = matches
        .value_of_os("target")
        .ok_or_else(|| String::from("Missing argument target"))?
        .into();

    let result = Arguments {
        source,
        target,
        reference,
    };

    if !result.source.exists() {
        return Err(format!("Source path {:?} must exist", result.source).into());
    }
    if !result.target.exists() {
        return Err(format!("Target path {:?} must exist", result.target).into());
    }
    if let Some(reference) = result.reference.as_ref() {
        if !reference.exists() {
            return Err(format!("If given reference path {:?} must exist", reference).into());
        }
    }

    Ok(result)
}

struct Arguments {
    source: PathBuf,
    target: PathBuf,
    reference: Option<PathBuf>,
}
