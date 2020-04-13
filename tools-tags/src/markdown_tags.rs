use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

use tools_utils::{Error, Result};

// TODO: use path instead of str
pub fn find_all_tags(root: &Path) -> impl Iterator<Item = Result<TaggedEntry>> {
    let mut path_iter = collect_mardown_documents(root);
    let mut current_tags: Option<Vec<TaggedEntry>> = None;

    std::iter::from_fn(move || -> Option<Result<TaggedEntry>> {
        loop {
            if current_tags.is_none() {
                let path = match path_iter.next()? {
                    Err(e) => return Some(Err(e)),
                    Ok(path) => path,
                };

                let tags = match parse_file(&path) {
                    Err(e) => return Some(Err(e)),
                    Ok(tags) => tags,
                };

                current_tags = Some(tags);
            }
            let current_tags_vec = current_tags.as_mut().unwrap();

            let tag = match current_tags_vec.pop() {
                None => {
                    current_tags = None;
                    continue;
                }
                Some(tag) => tag,
            };
            return Some(Ok(tag));
        }
    })
}

pub fn collect_mardown_documents(root: &Path) -> impl Iterator<Item = Result<PathBuf>> {
    fn is_non_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| !s.starts_with('.'))
            .unwrap_or(true)
    }
    let mut walker = WalkDir::new(root).into_iter().filter_entry(is_non_hidden);
    std::iter::from_fn(move || -> Option<Result<PathBuf>> {
        loop {
            let entry = walker.next()?;

            let entry = match entry {
                Err(e) => return Some(Err(Error::from(format!("Cannot read dir entry: {}", e)))),
                Ok(entry) => entry,
            };

            let is_markdown_file = entry
                .file_name()
                .to_str()
                .map(|s| s.ends_with(".md"))
                .unwrap_or(false);

            if !is_markdown_file {
                continue;
            }
            let result = entry.path().to_owned();
            return Some(Ok(result));
        }
    })
}

pub fn parse_file(path: &Path) -> Result<Vec<TaggedEntry>> {
    lazy_static! {
        static ref TAG_PATTERN: Regex =
            Regex::new(r"(^|\s)@(?P<tag>[^\s:]*)(:(?P<value>[^\s]*))?").unwrap();
    }

    let mut result = Vec::<TaggedEntry>::new();
    let mut section = String::from("");
    let mut line_idx = 1;

    let file = File::open(path).map_err(|e| format!("parse_file: could not open file: {}", e))?;
    let reader = BufReader::new(file);

    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("parse_file: could not read line: {}", e))?;
        if line.starts_with('#') {
            section = line.trim_matches('#').trim().to_owned();
            line_idx = idx + 1;
        } else if line.contains('@') {
            for cap in TAG_PATTERN.captures_iter(&line) {
                let tag = &cap["tag"];
                let value = cap.name("value").map(|m| m.as_str());
                result.push(TaggedEntry::new(path, line_idx, &section, tag, value));
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Clone)]
pub struct TaggedEntry {
    pub path: PathBuf,
    pub line: usize,
    pub section: String,
    pub tag: String,
    pub value: Option<String>,
}

impl TaggedEntry {
    pub fn new(path: &Path, line: usize, section: &str, tag: &str, value: Option<&str>) -> Self {
        Self {
            path: path.to_owned(),
            line,
            section: section.to_owned(),
            tag: tag.to_owned(),
            value: value.map(|s| s.to_owned()),
        }
    }
}
