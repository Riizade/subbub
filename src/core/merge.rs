use anyhow::Result;
use srtlib::Subtitles;

pub fn merge(primary: &Subtitles, secondary: &Subtitles) -> Result<Subtitles> {
    let mut merged = Subtitles::new();
    for subtitle in primary.into_iter() {
        merged.push(subtitle.clone());
    }

    for subtitle in secondary.into_iter() {
        const PREFIX: &str = r"{\an8}"; // places the subtitle at the top of the video instead of the bottom
        let mut altered_subtitle = subtitle.clone();
        altered_subtitle.text = format!("{PREFIX}{0}", altered_subtitle.text);
        merged.push(altered_subtitle);
    }
    // TODO: sort doesn't sort by time, it sorts by counter
    // need to take all the subtitles, sort by time, and give them new counters

    merged.sort();

    Ok(merged)
}
