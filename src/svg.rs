use crate::cli::SvgOpts;
use crate::BoxError;

pub fn main(opts: SvgOpts) -> Result<i32, BoxError> {
    let svg = r##"
    <?xml version="1.0" encoding="UTF-8"?>
        <svg version="1.1"
    xmlns="http://www.w3.org/2000/svg"
    xmlns:xlink="http://www.w3.org/1999/xlink"
    viewBox="0 -120 1446 1200">
        <symbol id="Foo-5/6.uni2269" overflow="visible"><path d="M100,334 L623,563 L623,619 L100,880 L100,793 L518,594 L100,420 Z M100,208 L622,208 L622,287 L100,287 Z M100,38 L622,38 L622,117 L100,117 Z M282,-93 L508,379 L436,413 L211,-59 Z"/></symbol>
        <use xlink:href="#Foo-5/6.uni2269" x="0" y="0"/>
        <use xlink:href="#Foo-5/6.uni2269" x="723" y="0"/>
    </svg>"##;

    println!("{}", svg);

    Ok(0)
}
