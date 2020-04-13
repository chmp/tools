use super::markdown_tags::{find_all_tags, TaggedEntry};
use super::utils::ResultsCollector;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};

pub struct TagList {
    root: PathBuf,
    tags: Vec<TaggedEntry>,
    errors: Vec<String>,
    on_update_callback: Box<dyn Fn(Arc<TagList>) + Send + Sync>,
    self_ref: LateInitWeak<TagList>,
}

// TOOD: add builder
impl TagList {
    pub fn new(
        root: impl AsRef<Path>,
        callback: impl Fn(Arc<TagList>) + Send + Sync + 'static,
    ) -> Arc<Self> {
        let root = root.as_ref();
        let (tags, errors) =
            find_all_tags(root).collect_results_transformed(|v| v, |e| e.to_string());

        let result = Self {
            root: root.to_owned(),
            tags,
            errors,
            on_update_callback: Box::new(callback),
            self_ref: LateInitWeak::new(),
        };

        let result = Arc::new(result);
        unsafe { result.self_ref.init(Arc::downgrade(&result)) };
        result
    }

    /// schedule an asynchronous update
    pub fn update(&self) {
        let self_ref = self
            .self_ref
            .upgrade()
            .expect("Internal error: detached TagList");
        (self.on_update_callback)(self_ref);
    }

    /// get a filtered list
    pub fn filter(&self, filter: &str) -> Vec<TaggedEntry> {
        self.tags
            .iter()
            .filter(|s| s.tag.contains(filter))
            .cloned()
            .collect::<Vec<_>>()
    }
}

// a special weak pointer that can be initialized late
struct LateInitWeak<T> {
    cell: Cell<Weak<T>>,
}

impl<T> LateInitWeak<T> {
    fn new() -> Self {
        Self {
            cell: Cell::new(Weak::new()),
        }
    }

    unsafe fn init(&self, value: Weak<T>) {
        self.cell.set(value);
    }

    fn get(&self) -> Weak<T> {
        let ptr = self.cell.as_ptr();
        unsafe { ptr.as_ref() }.unwrap().clone()
    }

    fn upgrade(&self) -> Option<Arc<T>> {
        self.get().upgrade()
    }
}

unsafe impl<T> Send for LateInitWeak<T> {}
unsafe impl<T> Sync for LateInitWeak<T> {}
