use std::{
    collections::{BinaryHeap, HashMap},
    ops::Range,
    path::PathBuf,
};

use mdbook_preprocessor::book::BookItem;

#[derive(Clone, Debug, Default)]
pub struct Rewrites {
    inner: HashMap<PathBuf, BinaryHeap<Rewrite>>,
}

#[derive(Debug, Clone)]
pub struct Rewrite {
    pub range: Range<usize>,
    pub replacement: String,
}

impl PartialEq for Rewrite {
    fn eq(&self, other: &Self) -> bool {
        self.range.start == other.range.start
    }
}

impl Eq for Rewrite {}

impl PartialOrd for Rewrite {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Rewrite {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.range.start.cmp(&other.range.start)
    }
}

impl Rewrites {
    pub fn at(&mut self, path: PathBuf) -> &mut BinaryHeap<Rewrite> {
        self.inner.entry(path).or_default()
    }

    pub fn apply(mut self, items: &mut [BookItem]) {
        self.apply_mut(items)
    }

    fn apply_mut(&mut self, items: &mut [BookItem]) {
        let chapters = items.iter_mut().filter_map(|i| match i {
            BookItem::Chapter(c) => Some(c),
            _ => None,
        });

        for chapter in chapters {
            let Some(path) = &chapter.path else {
                continue;
            };

            if let Some(rewrites) = self.inner.remove(path) {
                let content = &chapter.content;
                let mut output = String::new();
                let ordered = rewrites.into_sorted_vec();

                let mut last_copied = 0;
                for rewrite in ordered {
                    if rewrite.range.start != last_copied {
                        output.push_str(&content[last_copied..rewrite.range.start]);
                    }

                    output.push_str(&rewrite.replacement);
                    last_copied = rewrite.range.end;
                }

                output.push_str(&content[last_copied..]);

                chapter.content = output;
            }

            self.apply_mut(&mut chapter.sub_items);
        }
    }
}
