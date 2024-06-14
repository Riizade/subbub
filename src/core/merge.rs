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

    let mut merged_vec = merged.to_vec();
    // sort the subtitles by their start time
    merged_vec.sort_by_key(|s| s.start_time);
    // assign their numerical order according to their start time
    for (index, subtitle) in merged_vec.iter_mut().enumerate() {
        subtitle.num = index;
    }

    let merged = Subtitles::new_from_vec(merged_vec);

    Ok(merged)
}
