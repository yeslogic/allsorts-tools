use std::fs;

use allsorts::font_specimen::{self, SpecimenOptions};

use crate::cli::SpecimenOpts;
use crate::BoxError;

pub fn main(opts: SpecimenOpts) -> Result<i32, BoxError> {
    let specimen_options = SpecimenOptions {
        index: opts.index,
        sample_text: opts.sample_text,
    };
    let font_data = fs::read(&opts.font)?;
    let (head, body) = font_specimen::specimen(&opts.font, &font_data, specimen_options)?;

    println!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    {head}
</head>
<body>
    {body}
    <footer style="text-align: center">
        <img src="https://github.com/yeslogic/allsorts/raw/master/allsorts.svg?sanitize=1" width="32" style="vertical-align: middle" alt="">
        Generated with <a href="https://github.com/yeslogic/allsorts-tools">Allsorts</a>.
    </footer>
</body>
</html>"#
    );

    Ok(0)
}
