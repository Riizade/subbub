// this file contains functions to modify subtitles files

use anyhow::Result;
use scraper::Html;
use srtlib::Subtitles;

// strips HTML tags from subtitles, removing custom fonts, sizes, and colors
pub fn strip_html(subs: &mut Subtitles) -> Result<()> {
    for subtitle in subs.into_iter() {
        subtitle.text = strip_html_string(&subtitle.text);
    }
    Ok(())
}

fn strip_html_string(string: &str) -> String {
    let mut strings = vec![];
    let fragment = Html::parse_fragment(string);
    for node in fragment.tree {
        if let scraper::node::Node::Text(text) = node {
            strings.push(text.text.to_string());
        }
    }
    strings.join("")
}
