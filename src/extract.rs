use std::{ops::Range, path::PathBuf};

use indexmap::IndexMap;
use mdbook_preprocessor::book::{BookItem, Chapter};
use pulldown_cmark::{CowStr, Event, LinkType, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct Url<'a>(CowStr<'a>);

impl<'a> Url<'a> {
    pub fn new(value: CowStr<'a>) -> Option<Self> {
        value.contains(':').then_some(Self(value))
    }

    pub fn protocol(&self) -> &str {
        self.0.split_once(':').unwrap().0
    }

    pub fn value(&self) -> &str {
        self.0.split_once(':').unwrap().1
    }
}

#[derive(Debug, Clone)]
pub struct Link<'a> {
    pub url: Url<'a>,
    pub full_range: Range<usize>,
    pub title: CowStr<'a>,
    pub text: Vec<Event<'a>>,
}

impl<'a> Link<'a> {
    pub fn new(
        url: Url<'a>,
        full_range: Range<usize>,
        title: CowStr<'a>,
        text: Vec<Event<'a>>,
    ) -> Self {
        Self {
            url,
            full_range,
            title,
            text,
        }
    }
}

/// Extract elements recursively.
///
/// `map` will contain the paths to included
/// chapthers in summary order.
pub fn extract_elements_recursively<'a>(
    items: &'a Vec<BookItem>,
    map: &mut IndexMap<PathBuf, Vec<Link<'a>>>,
) {
    let chapters = items.iter().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    for chapter in chapters {
        let Some(path) = chapter.path.clone() else {
            continue;
        };

        let items = extract_elements(&chapter);
        map.insert(path, items);

        extract_elements_recursively(&chapter.sub_items, map);
    }
}

fn extract_elements(chapter: &Chapter) -> Vec<Link<'_>> {
    let mut elements = Vec::new();
    let content = &chapter.content;
    let mut parser = Parser::new(&content).into_offset_iter();

    while let Some((event, range)) = parser.next() {
        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            }) => {
                let Some(dest_url) = Url::new(dest_url) else {
                    continue;
                };

                let mut text = Vec::new();
                if link_type != LinkType::Autolink {
                    loop {
                        let (next, _) = parser.next().unwrap();

                        if next == Event::End(TagEnd::Link) {
                            break;
                        } else {
                            text.push(next);
                        }
                    }
                }

                elements.push(Link::new(dest_url, range, title, text));
            }
            _ => {}
        }
    }

    elements
}
