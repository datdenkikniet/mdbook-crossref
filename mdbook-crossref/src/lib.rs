use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle};
use anyhow::{Context, Result};
use mdbook_preprocessor::book::{Book, BookItem};
use pulldown_cmark::{CowStr, Event, LinkType, Parser, Tag, TagEnd};
use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
};

mod rewrite;

use crate::rewrite::{Rewrite, Rewrites};

#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
struct Crossref {
    url: String,
    supplement: Option<String>,
}

type LinkMap<'a> = HashMap<PathBuf, Vec<Link<'a>>>;

#[derive(Default)]
pub struct CrossrefPreprocessor;

impl CrossrefPreprocessor {
    pub fn rewrite_book(book: &mut Book) -> Result<()> {
        let mut map = Default::default();
        let mut rewrites = Default::default();

        extract_links(&book.items, &mut map);

        let crossrefs = rewrite_and_scan_labels(&mut rewrites, &mut map)?;
        rewrite_refs(&mut rewrites, &map, &crossrefs)?;

        rewrites.apply(&mut book.items);

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
    pub element_range: Range<usize>,
    pub title: CowStr<'a>,
    pub text: Option<&'a str>,
    pub full_source: &'a str,
    pub source_path: &'a Path,
}

impl<'a> Link<'a> {
    pub fn new(
        url: Url<'a>,
        element_range: Range<usize>,
        title: CowStr<'a>,
        text: Option<&'a str>,
        full_source: &'a str,
        source_path: &'a Path,
    ) -> Self {
        Self {
            url,
            element_range,
            title,
            text,
            full_source,
            source_path,
        }
    }
}

/// Extract elements recursively.
///
/// `map` will contain the paths to included
/// chapthers in summary order.
fn extract_links<'a>(items: &'a Vec<BookItem>, map: &mut LinkMap<'a>) {
    let chapters = items.iter().filter_map(|i| match i {
        BookItem::Chapter(c) => Some(c),
        _ => None,
    });

    for chapter in chapters {
        let Some(path) = chapter.path.as_ref() else {
            continue;
        };

        let items = extract_links_chapter(&chapter.content, path);
        map.insert(path.to_path_buf(), items);

        extract_links(&chapter.sub_items, map);
    }
}

fn extract_links_chapter<'a>(content: &'a str, path: &'a Path) -> Vec<Link<'a>> {
    let mut elements = Vec::new();
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

                elements.push(Link::new(dest_url, range, title, text, content, path));
            }
            _ => {}
        }
    }

    elements
}

fn rewrite_and_scan_labels(
    rewrites: &mut Rewrites,
    map: &LinkMap,
) -> Result<HashMap<String, Crossref>> {
    let mut known_crossrefs: HashMap<String, (Crossref, &Link<'_>)> = HashMap::new();
    let mut report = Vec::new();

    for (md_path, links) in map {
        let rewrites_path = rewrites.at(md_path.clone());
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

            if let Some((_, original_link)) = known_crossrefs.get(id) {
                report.push(
                    Level::ERROR
                        .primary_title(format!("Duplicate label '{id}'"))
                        .element(
                            Snippet::source(original_link.full_source)
                                .path(original_link.source_path.display().to_string())
                                .annotation(
                                    AnnotationKind::Context
                                        .span(original_link.element_range.clone())
                                        .label("Originally defined here"),
                                ),
                        )
                        .element(
                            Snippet::source(link.full_source)
                                .path(md_path.display().to_string())
                                .annotation(
                                    AnnotationKind::Primary
                                        .span(link.element_range.clone())
                                        .label("Error occurred here"),
                                ),
                        ),
                );

                continue;
            } else {
                known_crossrefs.insert(
                    id.to_string(),
                    (
                        Crossref {
                            url: format!("/{path}#{anchor}", path = md_path.display(), anchor = id),
                            supplement,
                        },
                        link,
                    ),
                );
            };

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
                range: link.element_range.clone(),
                replacement,
            });
        }
    }

    if !report.is_empty() {
        let renderer = Renderer::styled().decor_style(DecorStyle::Unicode);
        anstream::eprintln!("{}", renderer.render(&report));
        anyhow::bail!("Encountered errors");
    }

    let mapped = known_crossrefs
        .into_iter()
        .map(|(k, (v, _))| (k, v))
        .collect();

    Ok(mapped)
}

fn rewrite_refs(
    rewrites: &mut Rewrites,
    map: &LinkMap,
    crossrefs: &HashMap<String, Crossref>,
) -> Result<()> {
    // Rewrite all links
    for (md_path, links) in map {
        let rewrites = rewrites.at(md_path.clone());
        for link in links {
            if link.url.protocol() != "ref" {
                continue;
            }

            let Some(crossref) = crossrefs.get(link.url.value()) else {
                anyhow::bail!("Unknown reference `{}`", link.url.value());
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
                range: link.element_range.clone(),
                replacement,
            };

            rewrites.push(rewrite);
        }
    }

    Ok(())
}
