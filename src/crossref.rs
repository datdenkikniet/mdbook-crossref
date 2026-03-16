use std::collections::HashMap;

use anyhow::{Context, Result};
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

use crate::{CrossrefPreprocessor, rewrite::Rewrite};

#[derive(Debug, Clone)]
struct Crossref {
    url: String,
    supplement: Option<String>,
}

impl CrossrefPreprocessor<'_> {
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
                let replacement = if !link.text.is_empty() {
                    let mut replacement = format!(r#"<span id="{id}">"#);
                    pulldown_cmark::html::write_html_fmt(
                        &mut replacement,
                        link.text.iter().cloned(),
                    )
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

    pub fn create_crossref_rewrites(&mut self) -> Result<()> {
        let known_crossrefs = self.rewrite_and_scan_labels()?;
        self.rewrite_refs(&known_crossrefs)
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

                let text = if !link.text.is_empty() {
                    link.text.clone()
                } else if let Some(supp) = &crossref.supplement {
                    vec![Event::Text(CowStr::Boxed(
                        supp.to_string().into_boxed_str(),
                    ))]
                } else {
                    eprintln!("Cross-reference had neither supplement nor text");
                    continue;
                };

                let link_start = Event::Start(Tag::Link {
                    link_type: pulldown_cmark::LinkType::Inline,
                    dest_url: crossref.url.clone().into(),
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

                let rewrite = Rewrite {
                    range: link.full_range.clone(),
                    replacement: link_resolved,
                };

                rewrites.push(rewrite);
            }
        }

        Ok(())
    }
}
