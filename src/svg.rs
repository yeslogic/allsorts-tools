use xmlwriter::{XmlWriter, Options};

use crate::cli::SvgOpts;
use crate::BoxError;

pub fn main(opts: SvgOpts) -> Result<i32, BoxError> {
    let mut w = XmlWriter::new(Options::default());
    w.write_declaration();
    w.start_element("svg");
    w.write_attribute("version", "1.1");
    w.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    w.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    w.write_attribute("viewBox", "0 -120 1446 1200");

    w.start_element("symbol");
    w.write_attribute("id", "Foo-5/6.uni2269");
    w.write_attribute("overflow", "visible");

    w.start_element("path");
    w.write_attribute("d", "M100,334 L623,563 L623,619 L100,880 L100,793 L518,594 L100,420 Z M100,208 L622,208 L622,287 L100,287 Z M100,38 L622,38 L622,117 L100,117 Z M282,-93 L508,379 L436,413 L211,-59 Z");
    w.end_element(); // path
    w.end_element(); // symbol

    w.start_element("use");
    w.write_attribute("xlink:href", "#Foo-5/6.uni2269");
    w.write_attribute("x", "0");
    w.write_attribute("y", "0");
    w.end_element(); // use

    w.start_element("use");
    w.write_attribute("xlink:href", "#Foo-5/6.uni2269");
    w.write_attribute("x", "723");
    w.write_attribute("y", "0");
    w.end_element(); // use

    let svg = w.end_document();

    println!("{}", svg);

    Ok(0)
}
