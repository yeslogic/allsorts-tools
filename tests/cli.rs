use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn dump_glyph() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("allsorts")?;
    cmd.args(&["dump", "-g", "1", "tests/Basic-Regular.ttf"]);
    let expected = r#"Parsed(
    Simple(
        SimpleGlyph {
            bounding_box: BoundingBox {
                x_min: 158,
                x_max: 1082,
                y_min: 0,
                y_max: 1358,
            },
            end_pts_of_contours: [
                22,
                35,
                44,
            ],"#;
    cmd.assert()
        .success()
        .stdout(predicate::str::starts_with(expected));

    Ok(())
}

#[test]
fn dump_empty_glyph() -> Result<(), Box<dyn std::error::Error>> {
    // Glyph 112 is .null
    let mut cmd = Command::cargo_bin("allsorts")?;
    cmd.args(&["dump", "-g", "112", "tests/Basic-Regular.ttf"]);
    let expected = r#"Parsed(
    Empty(
        EmptyGlyph {
            phantom_points: None,
        },
    ),
)
"#;
    cmd.assert().success().stdout(expected);

    Ok(())
}
