// this file contains functions to modify subtitles files

use anyhow::Result;
use scraper::Html;
use srtlib::Subtitles;

pub fn clean_subtitles(subs: &mut Subtitles) -> Result<()> {
    strip_html(subs)?;
    remove_bracketed_info(subs)?;
    Ok(())
}

// strips HTML tags from subtitles, removing custom fonts, sizes, and colors
fn strip_html(subs: &mut Subtitles) -> Result<()> {
    for subtitle in subs.into_iter() {
        subtitle.text = strip_html_string(&subtitle.text);
    }
    Ok(())
}

fn remove_bracketed_info(subs: &mut Subtitles) -> Result<()> {
    for subtitle in subs.into_iter() {
        subtitle.text = remove_bracketed_info_from_string(&subtitle.text);
    }
    Ok(())
}

fn remove_bracketed_info_from_string(string: &str) -> String {
    let mut result = String::new();
    let mut skip = 0;
    for c in string.chars() {
        match c {
            '<' | '{' | '[' => skip += 1,
            '>' | '}' | ']' => {
                if skip > 0 {
                    skip -= 1;
                }
            }
            _ => {
                if skip == 0 {
                    result.push(c);
                }
            }
        }
    }
    result
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

pub fn shift_seconds(subtitles: &Subtitles, seconds: f32) -> Result<Subtitles> {
    let mut shifted_subs = subtitles.clone().to_vec();
    let iseconds = seconds as i32;
    let imillis = ((seconds - iseconds as f32) * 1000.0) as i32;

    for subtitle in shifted_subs.iter_mut() {
        subtitle.add_seconds(iseconds);
        subtitle.add_milliseconds(imillis);
    }

    Ok(Subtitles::new_from_vec(shifted_subs))
}
