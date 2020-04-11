mod arxiv;

use lazy_static::lazy_static;
use regex::Regex;
use std::{
    env,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use reqwest::header::USER_AGENT;
use tools_utils::{run_main, Result};

use arxiv::{is_arxiv_paper, parse_arxiv_metadata};

fn main() {
    run_main(main_impl);
}

fn main_impl() -> Result<i32> {
    let args = parse_args()?;
    println!("Sort papers");
    println!("Source: {:?}", args.source);
    println!("Target: {:?}", args.target);

    process_directory(&args.source, &args.target)?;
    Ok(0)
}

fn parse_args() -> Result<Arguments> {
    let args = env::args_os().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(String::from("Usage: tools papers SRC DST").into());
    }

    let result = Arguments {
        source: args[1].clone().into(),
        target: args[2].clone().into(),
    };

    Ok(result)
}

struct Arguments {
    source: PathBuf,
    target: PathBuf,
}

fn process_directory<P, Q>(root: P, target: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let root = root.as_ref();
    let target = target.as_ref();
    for entry in root.read_dir().map_err(|e| format!("Cannot read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Cannot read item information: {}", e))?;
        let path = entry.path();

        let paper = parse_paper_path(&path);

        // println!("{}: {}", filename, is_arxiv_paper(stem));
        match paper {
            Paper::Arxiv { path, id } => {
                // TODO: port to API and use xml response instead of parsing text format
                // let url = format!("http://export.arxiv.org/api/query?id_list={}", id);
                let url = format!("https://export.arxiv.org/abs/{}?fmt=txt", id);
                
                let client = reqwest::blocking::Client::new();
                let metadata = client.get(&url)
                    .header(USER_AGENT, "ArxivPaperTools/1.0")
                    .send()
                    .and_then(|r| r.text())
                    .map_err(|e| format!("Error during download of metadata: {}", e))?;
                
                let metadata = parse_arxiv_metadata(&metadata)
                    .ok_or_else(|| format!("Cannot parse metadata for {}. \n===\n{}", id, metadata))?;
                let new_path = metadata.get("Title")
                    .ok_or_else(|| format!("Missing title meta data for {}", id))?;
                let new_path = normalize_title(new_path);
                let new_path = format!("{}_{}.pdf", id, new_path);
                let new_path = target.join(new_path);
                println!("{:?} -> {:?}", path, new_path);
                std::fs::rename(path, new_path)
                    .map_err(|e| format!("Cannot rename path: {}", e))?;

                // sleep to conform with Arxiv Usage guidelines
                thread::sleep(Duration::from_millis(250));
            }
            Paper::Unknown { path } => {
                println!("ignore {:?} ", path);
            }
        }
    }

    Ok(())
}

/// Normalize a paper title such that it is suitable for renaming the file  
fn normalize_title(s: &str) -> String {
    lazy_static! {
        static ref PATTERN: Regex = Regex::new(r##"\s+"##).unwrap();
    }
    let s = s.replace(
        |c: char| !c.is_alphanumeric() && !c.is_whitespace() && c != '-',
        "",
    );
    let s = s.to_lowercase();
    let s = PATTERN.replace_all(&s, "_");
    s.to_string()
}

enum Paper<'a> {
    Unknown { path: &'a Path },
    Arxiv { path: &'a Path, id: &'a str },
}

fn parse_paper_path<P: AsRef<Path>>(path: &P) -> Paper {
    let path = path.as_ref();
    let stem = path.file_stem().and_then(|s| s.to_str());
    if stem.is_none() {
        return Paper::Unknown { path };
    }
    let stem = stem.unwrap();

    if is_arxiv_paper(stem) {
        Paper::Arxiv { path, id: stem }
    } else {
        Paper::Unknown { path }
    }
}
