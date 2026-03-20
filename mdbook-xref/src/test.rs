use std::path::PathBuf;

use crate::{Link, Url, rewrite_and_scan_labels};

#[test]
pub fn duplicate_label_produces_error() {
    let links = [(
        PathBuf::from("path"),
        vec![
            Link {
                url: Url::new("label:duplicate".into()).unwrap(),
                full_range: 0..0,
                title: "title".into(),
                text: None,
            },
            Link {
                url: Url::new("label:duplicate".into()).unwrap(),
                full_range: 0..0,
                title: "title".into(),
                text: None,
            },
        ],
    )]
    .into_iter()
    .collect();

    let result = rewrite_and_scan_labels(&mut Default::default(), &links);

    assert!(result.is_err());
}
