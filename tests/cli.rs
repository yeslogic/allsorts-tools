use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

#[test]
fn dump_glyph() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("allsorts")?;
    cmd.args(&["dump", "-g", "1", "tests/Basic-Regular.ttf"]);
    cmd.assert().success().stdout(predicate::str::starts_with(
        "Parsed(\n    Glyph {\n        number_of_contours: 3,",
    ));

    Ok(())
}

#[test]
fn dump_empty_glyph() -> Result<(), Box<dyn std::error::Error>> {
    // Glyph 112 is .null
    let mut cmd = Command::cargo_bin("allsorts")?;
    cmd.args(&["dump", "-g", "112", "tests/Basic-Regular.ttf"]);
    cmd.assert().success().stdout("Empty\n");

    Ok(())
}
