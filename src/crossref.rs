use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use mdbook_preprocessor::book::SectionNumber;

use anyhow::{Context, Result};
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

use crate::{
    Element, Rewriter,
    extract::Heading,
    rewrite::{Rewrite, Rewrites},
};

#[derive(Debug, Clone)]
enum Supplement {
    Section(SectionNumber),
    Figure(usize),
}

impl std::fmt::Display for Supplement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Supplement::Section(section_number) => {
                write!(f, "Section ")?;

                let mut numbers = section_number.iter().peekable();

                while let Some(num) = numbers.next() {
                    write!(f, "{num}")?;

                    if numbers.peek().is_some() {
                        f.write_char('.')?;
                    }
                }

                Ok(())
            }
            Supplement::Figure(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
struct Crossref<'a> {
    path: &'a Path,
    anchor: &'a str,
    supplement: Option<Supplement>,
}

impl<'a> Crossref<'a> {
    pub fn url(&self) -> String {
        format!(
            "/{path}#{anchor}",
            path = self.path.display(),
            anchor = self.anchor
        )
    }
}

#[derive(Default)]
struct Numbering([u32; 32]);

impl Numbering {
    pub fn next(&mut self, level: usize) -> SectionNumber {
        self.0[level - 1] += 1;

        self.0.iter_mut().skip(level).for_each(|v| *v = 0);

        let output: Vec<_> = self.0.iter().take(level).copied().collect();

        SectionNumber::new(output)
    }
}

impl Rewriter {
    pub fn create_crossref_rewrites(
        &self,
        map: &IndexMap<PathBuf, Vec<Element>>,
        rewrites: &mut Rewrites,
    ) -> Result<()> {
        let mut known_links = HashMap::new();

        let mut section_numbering = Numbering::default();

        for (md_path, elements) in map {
            let mut new_numbering = None;

            let rewrites_path = rewrites.at(md_path.clone());
            for element in elements {
                match element {
                    Element::AutolinkedHeading {
                        heading:
                            Heading {
                                level,
                                source,
                                text,
                            },
                        link_range,
                        link_url,
                    } => {
                        if link_url.protocol() != "label" {
                            continue;
                        }

                        let label_name = link_url.value();
                        let numbering = section_numbering.next(*level);
                        let supplement = Supplement::Section(numbering.clone());

                        known_links.insert(
                            label_name,
                            Crossref {
                                path: md_path.as_ref(),
                                anchor: label_name,
                                supplement: Some(supplement),
                            },
                        );

                        // Replace with mdbook-native ID format
                        rewrites_path.push(Rewrite {
                            range: link_range.clone(),
                            replacement: format!("{{ #{label_name} }}"),
                        });

                        let (source_level, source_range) = source.clone().unwrap();

                        let prefix: String =
                            std::iter::repeat_n('#', source_level as usize).collect();

                        let replacement = format!("{prefix} {numbering} {text}");

                        rewrites_path.push(Rewrite {
                            range: source_range,
                            replacement,
                        })
                    }
                    Element::Heading(Heading {
                        level,
                        source,
                        text,
                    }) => {
                        let numbering = section_numbering.next(*level);

                        if let Some((source_level, source_range)) = source.clone() {
                            let prefix: String =
                                std::iter::repeat_n('#', source_level as usize).collect();
                            let replacement = format!("{prefix} {numbering} {text}");
                            rewrites_path.push(Rewrite {
                                range: source_range,
                                replacement,
                            })
                        } else {
                            new_numbering = Some(numbering);
                        }
                    }
                    Element::Link(link) => {
                        if link.url.protocol() != "label" || link.text.is_empty() {
                            continue;
                        }

                        let id = link.url.value();

                        known_links.insert(
                            id,
                            Crossref {
                                path: md_path.as_ref(),
                                anchor: id,
                                supplement: None,
                            },
                        );

                        let title = if !link.title.is_empty() {
                            format!(r#"title="{title}""#, title = link.title)
                        } else {
                            "".to_string()
                        };

                        // Render in-place
                        let mut replacement = format!(r#"<span id="{id}" {title}>"#);
                        pulldown_cmark::html::write_html_fmt(
                            &mut replacement,
                            link.text.iter().cloned(),
                        )
                        .context("failed to render labelled text")?;
                        replacement.push_str("</span>");

                        rewrites_path.push(Rewrite {
                            range: link.full_range.clone(),
                            replacement,
                        });
                    }
                    _ => continue,
                };
            }

            if let Some(new_numbering) = new_numbering {
                rewrites.set_numbering(md_path.to_path_buf(), new_numbering);
            }
        }

        for (md_path, elements) in map {
            let rewrites = rewrites.at(md_path.clone());
            for element in elements {
                let rewrite = match element {
                    Element::Link(link) => {
                        if link.url.protocol() != "ref" {
                            continue;
                        }

                        let Some(crossref) = known_links.get(link.url.value()) else {
                            eprintln!("Unknown reference `{}`", link.url.value());
                            continue;
                        };

                        let text = if !link.text.is_empty() {
                            link.text.clone()
                        } else if let Some(supp) = &crossref.supplement {
                            vec![Event::Text(CowStr::Boxed(
                                supp.to_string().into_boxed_str(),
                            ))]
                        } else {
                            eprintln!("Link had neither supplement nor text");
                            continue;
                        };

                        let link_start = Event::Start(Tag::Link {
                            link_type: pulldown_cmark::LinkType::Inline,
                            dest_url: CowStr::Boxed(crossref.url().into_boxed_str()),
                            title: link.title.clone(),
                            id: CowStr::Borrowed(""),
                        });

                        let events = Some(link_start)
                            .into_iter()
                            .chain(text)
                            .chain(Some(Event::End(TagEnd::Link)));

                        let mut link_resolved = String::new();
                        pulldown_cmark_to_cmark::cmark(events, &mut link_resolved)
                            .context("failed to format cross-reference")?;

                        Rewrite {
                            range: link.full_range.clone(),
                            replacement: link_resolved,
                        }
                    }
                    _ => continue,
                };

                rewrites.push(rewrite);
            }
        }

        Ok(())
    }
}
