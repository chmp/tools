use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use regex::Regex;
use walkdir::{DirEntry, Result as WalkdirResult, WalkDir};

use tools_utils::{Error, Result};

fn main() -> Result<()> {
    let args = env::args().collect::<Vec<_>>();
    println!("Args: {:?}", args);

    if args.len() != 2 {
        panic!("Wrong arguments: tag-browser DIRECTORY");
    }

    let tags = find_all_tags(&args[1]);
    let mut tags_by_name = HashMap::<String, Vec<TaggedEntry>>::new();

    for tag in tags {
        let tag = tag?;
        if !tags_by_name.contains_key(&tag.tag) {
            tags_by_name.insert(tag.tag.to_owned(), Vec::new());
        }

        tags_by_name.get_mut(&tag.tag).unwrap().push(tag);
    }

    for (tag, entries) in sorted(tags_by_name) {
        println!("{}: {} items", tag, entries.len());
    }

    Ok(())
}

fn sorted(tags_by_name: HashMap<String, Vec<TaggedEntry>>) -> Vec<(String, Vec<TaggedEntry>)> {
    let mut result = tags_by_name.into_iter().collect::<Vec<_>>();
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    result
}

// TODO: use path instead of str
fn find_all_tags(root: &str) -> impl Iterator<Item = Result<TaggedEntry>> {
    let path_iter = collect_mardown_documents(root);
    TaggedEntryIterator {
        path_iter,
        current_tags: None,
    }
}

struct TaggedEntryIterator<I: Iterator<Item = Result<PathBuf>>> {
    path_iter: I,
    current_tags: Option<Vec<TaggedEntry>>,
}

impl<I: Iterator<Item = Result<PathBuf>>> Iterator for TaggedEntryIterator<I> {
    type Item = Result<TaggedEntry>;

    fn next(&mut self) -> Option<Result<TaggedEntry>> {
        loop {
            if self.current_tags.is_none() {
                let path = match self.path_iter.next()? {
                    Err(e) => return Some(Err(e)),
                    Ok(path) => path,
                };

                let tags = match parse_file(&path) {
                    Err(e) => return Some(Err(e)),
                    Ok(tags) => tags,
                };

                self.current_tags = Some(tags);
            }
            let current_tags = self.current_tags.as_mut().unwrap();

            let tag = match current_tags.pop() {
                None => {
                    self.current_tags = None;
                    continue;
                }
                Some(tag) => tag,
            };
            return Some(Ok(tag));
        }
    }
}

fn collect_mardown_documents(root: &str) -> impl Iterator<Item = Result<PathBuf>> {
    fn is_non_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| !s.starts_with('.'))
            .unwrap_or(true)
    }
    let walker = WalkDir::new(root).into_iter().filter_entry(is_non_hidden);
    MarkdownIterator { iter: walker }
}

struct MarkdownIterator<I: Iterator<Item = WalkdirResult<DirEntry>>> {
    iter: I,
}

impl<I: Iterator<Item = WalkdirResult<DirEntry>>> Iterator for MarkdownIterator<I> {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Result<PathBuf>> {
        loop {
            let entry = self.iter.next()?;

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
    }
}

fn parse_file(path: &Path) -> Result<Vec<TaggedEntry>> {
    lazy_static! {
        static ref TAG_PATTERN: Regex =
            Regex::new(r"(^|\s)@(?P<tag>[^\s:]*)(:(?P<value>[^\s]*))?").unwrap();
    }

    let mut result = Vec::<TaggedEntry>::new();
    let mut section = String::from("");

    let file = File::open(path).map_err(|e| format!("parse_file: could not open file: {}", e))?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.map_err(|e| format!("parse_file: could not read line: {}", e))?;
        if line.starts_with('#') {
            section = line.trim_matches('#').trim().to_owned();
        } else if line.contains('@') {
            for cap in TAG_PATTERN.captures_iter(&line) {
                let tag = &cap["tag"];
                let value = cap.name("value").map(|m| m.as_str());
                result.push(TaggedEntry::new(path, &section, tag, value));
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Clone)]
struct TaggedEntry {
    path: PathBuf,
    section: String,
    tag: String,
    value: Option<String>,
}

impl TaggedEntry {
    fn new(path: &Path, section: &str, tag: &str, value: Option<&str>) -> Self {
        Self {
            path: path.to_owned(),
            section: section.to_owned(),
            tag: tag.to_owned(),
            value: value.map(|s| s.to_owned()),
        }
    }
}
