use anyhow::{Context, Result};
use indexmap::IndexMap;
use mdbook_preprocessor::book::{Book, BookItem, Chapter};
use pulldown_cmark::{CowStr, Event, LinkType, Parser, Tag, TagEnd};
use std::{collections::HashMap, ops::Range, path::PathBuf};

mod rewrite;

use crate::rewrite::{Rewrite, Rewrites};

#[derive(Debug, Clone)]
struct Crossref {
    url: String,
    supplement: Option<String>,
}

pub struct CrossrefPreprocessor<'a> {
    rewrites: Rewrites,
    map: IndexMap<PathBuf, Vec<Link<'a>>>,
}

impl CrossrefPreprocessor<'_> {
    pub fn rewrite_book(book: &mut Book) -> Result<()> {
        let mut map = IndexMap::new();
        extract_links(&book.items, &mut map);

        let mut me = CrossrefPreprocessor {
            rewrites: Default::default(),
            map,
        };

        let crossrefs = me.rewrite_and_scan_labels()?;
        me.rewrite_refs(&crossrefs)?;

        me.rewrites.apply(&mut book.items);

        Ok(())
    }

    fn rewrite_and_scan_labels(&mut self) -> Result<HashMap<String, Crossref>> {
        let mut known_crossrefs = HashMap::new();

        for (md_path, links) in &self.map {
            let rewrites_path = self.rewrites.at(md_path.clone());
            for link in links {
                if link.url.protocol() != "label" {
                    continue;
                }

                let id = link.url.value();

                let supplement = if !link.title.is_empty() {
                    Some(link.title.to_string())
                } else {
                    None
                };

                known_crossrefs.insert(
                    id.to_string(),
                    Crossref {
                        url: format!("/{path}#{anchor}", path = md_path.display(), anchor = id),
                        supplement,
                    },
                );

                // Render in-place
                let replacement = if let Some(text) = link.text {
                    let mut replacement = format!(r#"<span id="{id}">"#);
                    let output = pulldown_cmark::Parser::new(text);
                    pulldown_cmark::html::write_html_fmt(&mut replacement, output)
                        .context("failed to render labeled text")?;
                    replacement.push_str("</span>");
                    replacement
                } else {
                    "".to_string()
                };

                rewrites_path.push(Rewrite {
                    range: link.full_range.clone(),
                    replacement,
                });
            }
        }

        Ok(known_crossrefs)
    }

    fn rewrite_refs(&mut self, crossrefs: &HashMap<String, Crossref>) -> Result<()> {
        // Rewrite all links
        for (md_path, links) in &self.map {
            let rewrites = self.rewrites.at(md_path.clone());
            for link in links {
                if link.url.protocol() != "ref" {
                    continue;
                }

                let Some(crossref) = crossrefs.get(link.url.value()) else {
                    eprintln!("Unknown reference `{}`", link.url.value());
                    continue;
                };

                let supplement = if let Some(text) = link.text {
                    text
                } else if let Some(supp) = &crossref.supplement {
                    supp.as_ref()
                } else {
                    eprintln!("Cross-reference had neither supplement nor text");
                    continue;
                };

                let replacement = format!("[{supplement}]({url})", url = crossref.url);

                let rewrite = Rewrite {
                    range: link.full_range.clone(),
                    replacement,
                };

                rewrites.push(rewrite);
            }
        }

        Ok(())
    }
}

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
    pub text: Option<&'a str>,
}

impl<'a> Link<'a> {
    pub fn new(
        url: Url<'a>,
        full_range: Range<usize>,
        title: CowStr<'a>,
        text: Option<&'a str>,
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
fn extract_links<'a>(items: &'a Vec<BookItem>, map: &mut IndexMap<PathBuf, Vec<Link<'a>>>) {
    let chapters = items.iter().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    for chapter in chapters {
        let Some(path) = chapter.path.clone() else {
            continue;
        };

        let items = extract_links_chapter(&chapter);
        map.insert(path, items);

        extract_links(&chapter.sub_items, map);
    }
}

fn extract_links_chapter(chapter: &Chapter) -> Vec<Link<'_>> {
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

                let text = if link_type != LinkType::Autolink {
                    let (mut current, mut current_range) = parser.next().unwrap();
                    let start = current_range.start;
                    let mut end = current_range.end;
                    loop {
                        if current == Event::End(TagEnd::Link) {
                            break;
                        } else {
                            end = current_range.end;
                        }

                        let (next, range) = parser.next().unwrap();
                        current = next;
                        current_range = range;
                    }

                    Some(&content[start..end])
                } else {
                    None
                };

                elements.push(Link::new(dest_url, range, title, text));
            }
            _ => {}
        }
    }

    elements
}
