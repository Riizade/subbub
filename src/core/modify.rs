// this file contains functions to modify subtitles files

use anyhow::Result;
use scraper::Html;
use srtlib::{Subtitle, Subtitles, Timestamp};

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

trait Seconds {
    fn seconds(&self) -> f32;
}

impl Seconds for Timestamp {
    fn seconds(&self) -> f32 {
        let (hours, minutes, seconds, milliseconds) = self.get();
        return (hours as u32 * 3600) as f32
            + (minutes * 60) as f32
            + seconds as f32
            + (milliseconds as f32) / 1000.0;
    }
}

pub fn shift_seconds(subtitles: &Subtitles, seconds: f32) -> Result<Subtitles> {
    let iseconds = seconds as i32;
    let imillis = ((seconds - iseconds as f32) * 1000.0) as i32;
    let shifted_subs: Vec<Subtitle> = subtitles
        .clone()
        .to_vec()
        .iter()
        .map(|subs| {
            let sub_stamp = subs.start_time.seconds();
            // prevent underflowing
            // if we're subtracting time (e.g., -3 seconds), we want to avoid including any subs that start before the 3 second mark
            // we multiply by -1.0 instead of using abs() because abs would strip subtitles when adding time (displaying later), which we don't want
            if sub_stamp < -1.0 * seconds {
                None
            } else {
                let mut shifted_subs = subs.clone();
                shifted_subs.add_seconds(iseconds);
                shifted_subs.add_milliseconds(imillis);
                Some(shifted_subs)
            }
        })
        .flatten()
        .collect();

    Ok(Subtitles::new_from_vec(shifted_subs))
}
