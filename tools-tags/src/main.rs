mod markdown_tags;
mod tag_list;
mod utils;

use std::{env, path::PathBuf, process::Command, sync::Arc};

use cursive::{
    event::Key,
    traits::*,
    views::{Button, EditView, LinearLayout, ListView, TextView, ViewRef},
    Cursive,
};
use cursive_aligned_view::Alignable;

use tag_list::TagList;
use tools_utils::Result;
use utils::Ignorable;

fn main() -> Result<()> {
    let args = parse_args()?;

    let mut siv = Cursive::default();
    let cb_sink = siv.cb_sink().clone();

    let callback = move |t: Arc<TagList>| {
        let cb: Box<dyn FnOnce(&mut Cursive) + Send> = Box::new(move |s| {
            let search_box: ViewRef<EditView> = s.find_name("search_box").unwrap();
            let mut list: ViewRef<ListView> = s.find_name("list").unwrap();

            let entries = t.filter(search_box.get_content().as_ref());

            list.clear();
            for entry in entries {
                let title = format!(
                    "@{}: {}#{}",
                    entry.tag,
                    entry
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?"),
                    entry.section
                );
                let tag = "";
                let mut target = entry.path.into_os_string();
                target.push(":");
                target.push(entry.line.to_string());

                list.add_child(
                    tag,
                    Button::new(title, move |_s| {
                        Command::new("cmd")
                            .arg("/C")
                            .arg("code.cmd")
                            .arg("-g")
                            .arg(&target)
                            .spawn()
                            .unwrap()
                            .wait()
                            .unwrap();
                    })
                    .align_center_left(),
                );
            }
        });

        cb_sink.send(cb).unwrap();
    };

    let tag_list = TagList::new(&args.root, callback);
    let layout = LinearLayout::vertical()
        .child(TextView::new(format!("root: {:?}", args.root)))
        .child(
            EditView::new()
                .with(|edit_view| {
                    let tag_list = tag_list.clone();
                    edit_view.set_on_edit(move |_, _, _| tag_list.update());
                    edit_view.set_on_submit(|s, _| s.focus_name("list").ignore())
                })
                .with_name("search_box"),
        )
        .child(
            LinearLayout::horizontal()
                .child(Button::new("Refresh", |_s| {}))
                .child(Button::new("Quit", |s| s.quit())),
        )
        .child(ListView::new().with_name("list"));

    let layout = layout.align_top_left();

    siv.add_layer(layout);
    siv.add_global_callback('q', |s| s.quit());
    siv.add_global_callback(Key::Esc, |s| s.focus_name("search_box").ignore());

    tag_list.update();

    siv.run();

    Ok(())
}

fn parse_args() -> Result<Arguments> {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| "Wrong arguments. Usage: tools tags DIRECTORY")?;
    let result = Arguments { root };

    Ok(result)
}

struct Arguments {
    root: PathBuf,
}

/*
fn old_main() -> Result<()> {
    let args = parse_args()?;

    let tags = markdown_tags::find_all_tags(&args.root);
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
}*/
