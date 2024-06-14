use anyhow::Result;
use srtlib::{Subtitle, Subtitles};

pub fn merge(primary: &Subtitles, secondary: &Subtitles) -> Result<Subtitles> {
    // TODO: check for existing {\an8}, etc and ensure that subtitles do not overlap

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

fn modify_positioning(sub: &mut Subtitle, primary: bool) -> Result<()> {
    // ass/ssa specification: http://www.tcax.org/docs/ass-specs.htm
    // in particular:

    // \a<alignment>            <alignment> is a number specifying the onscreen alignment/positioning of a subtitle.
    // A value of 1 specifies a left-justified subtitle
    // A value of 2 specifies a centered subtitle
    // A value of 3 specifies a right-justified subtitle
    // Adding 4 to the value specifies a "Toptitle"
    // Adding 8 to the value specifies a "Midtitle"
    // 0 or nothing resets to the style default (which is usually 2)

    // eg. {\a1}This is a left-justified subtitle
    //       {\a2}This is a centered subtitle
    //       {\a3}This is a right-justified subtitle
    //       {\a5}This is a left-justified toptitle
    //       {\a11}This is a right-justified midtitle
    // Only the first appearance counts.

    // \an<alignment>         numpad layout
    // Only the first appearance counts.
    todo!();
    Ok(())
}
