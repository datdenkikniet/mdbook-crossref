use std::{ops::Range, path::PathBuf};

use indexmap::IndexMap;
use mdbook_preprocessor::book::{BookItem, Chapter};
use pulldown_cmark::{
    CowStr, Event, HeadingLevel, LinkType, Parser, Tag, TagEnd, TextMergeWithOffset,
};

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
pub struct CodeBlock<'a> {
    pub info: CowStr<'a>,
    pub contents: CowStr<'a>,
    pub full_range: Range<usize>,
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

#[derive(Debug, Clone)]
pub struct Heading<'a> {
    pub level: usize,
    pub source: Option<(HeadingLevel, Range<usize>)>,
    pub text: &'a str,
}

#[derive(Debug, Clone)]
pub enum Element<'a> {
    CodeBlock(CodeBlock<'a>),
    AutolinkedHeading {
        heading: Heading<'a>,
        link_range: Range<usize>,
        link_url: Url<'a>,
    },
    Heading(Heading<'a>),
    Link(Link<'a>),
}

/// Extract elements recursively.
///
/// `map` will contain the paths to included
/// chapthers in summary order.
pub fn extract_elements_recursively<'a>(
    items: &'a Vec<BookItem>,
    map: &mut IndexMap<PathBuf, Vec<Element<'a>>>,
) {
    let chapters = items.iter().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    for chapter in chapters {
        let Some(path) = chapter.path.clone() else {
            continue;
        };

        let mut items = vec![Element::Heading(Heading {
            level: chapter.number.as_ref().unwrap().len(),
            text: &chapter.name,
            source: None,
        })];

        extract_elements(&chapter, &mut items);
        map.insert(path, items);

        extract_elements_recursively(&chapter.sub_items, map);
    }
}

fn extract_elements<'a>(chapter: &'a Chapter, elements: &mut Vec<Element<'a>>) {
    let content = &chapter.content;
    let parser = Parser::new(&content).into_offset_iter();
    let mut parser = TextMergeWithOffset::new(parser);

    while let Some((event, range)) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(info))) => {
                let text = if let Some((Event::Text(text), _)) = parser.next() {
                    text
                } else {
                    CowStr::Borrowed("")
                };

                elements.push(Element::CodeBlock(CodeBlock {
                    info,
                    contents: text,
                    full_range: range,
                }));

                assert!(matches!(
                    parser.next(),
                    Some((Event::End(TagEnd::CodeBlock), _))
                ));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let mut text_start = None;

                let mut last_link_start = None;
                let auto_link_label = loop {
                    let Some((next, next_range)) = parser.next() else {
                        break None;
                    };

                    if text_start.is_none() {
                        text_start = Some(next_range.start);
                    }

                    match next {
                        Event::Start(Tag::Link {
                            link_type: LinkType::Autolink,
                            dest_url,
                            ..
                        }) => {
                            last_link_start = Some((dest_url, next_range));
                        }
                        Event::End(TagEnd::Link) => {
                            let last_link_start = last_link_start.take();
                            if parser.next().is_some_and(|(event, _)| {
                                event == Event::End(TagEnd::Heading(level))
                            }) {
                                break last_link_start;
                            }
                        }
                        Event::End(TagEnd::Heading(_)) => break None,
                        _ => {
                            continue;
                        }
                    }
                };

                if let Some((dest_url, link_range)) = auto_link_label {
                    let heading_no_link_range = range.start..link_range.start;
                    let text_data = text_start.unwrap()..link_range.start;

                    let Some(dest_url) = Url::new(dest_url) else {
                        continue;
                    };

                    let heading = Heading {
                        level: chapter.number.as_ref().unwrap().len() + level as usize,
                        text: &content[text_data],
                        source: Some((level, heading_no_link_range)),
                    };

                    elements.push(Element::AutolinkedHeading {
                        heading,
                        link_range,
                        link_url: dest_url,
                    });
                } else {
                    let text_data = text_start.unwrap()..range.end;

                    let heading = Heading {
                        level: chapter.number.as_ref().unwrap().len() + level as usize,
                        text: &content[text_data],
                        source: Some((level, range)),
                    };

                    elements.push(Element::Heading(heading));
                }
            }
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

                elements.push(Element::Link(Link::new(dest_url, range, title, text)))
            }
            _ => {}
        }
    }
}
