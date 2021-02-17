use allsorts::glyph_position::TextDirection;

mod rtl_tags {
    use allsorts::tag;

    pub const ARAB: u32 = tag!(b"arab"); // Arabic
    pub const HEBR: u32 = tag!(b"hebr"); // Hebrew
    pub const SYRC: u32 = tag!(b"syrc"); // Syriac
    pub const THAA: u32 = tag!(b"thaa"); // Thaana
    pub const CPRT: u32 = tag!(b"cprt"); // Cypriot Syllabary
    pub const KHAR: u32 = tag!(b"khar"); // Kharosthi
    pub const PHNX: u32 = tag!(b"phnx"); // Phoenician
    pub const NKO: u32 = tag!(b"nko "); // N'Ko
    pub const LYDI: u32 = tag!(b"lydi"); // Lydian
    pub const AVST: u32 = tag!(b"avst"); // Avestan
    pub const ARMI: u32 = tag!(b"armi"); // Imperial Aramaic
    pub const PHLI: u32 = tag!(b"phli"); // Inscriptional Pahlavi
    pub const PRTI: u32 = tag!(b"prti"); // Inscriptional Parthian
    pub const SARB: u32 = tag!(b"sarb"); // Old South Arabian
    pub const ORKH: u32 = tag!(b"orkh"); // Old Turkic, Orkhon Runic
    pub const SAMR: u32 = tag!(b"samr"); // Samaritan
    pub const MAND: u32 = tag!(b"mand"); // Mandaic, Mandaean
    pub const MERC: u32 = tag!(b"merc"); // Meroitic Cursive
    pub const MERO: u32 = tag!(b"mero"); // Meroitic Hieroglyphs

    // Unicode 7.0 (not listed on http://www.microsoft.com/typography/otspec/scripttags.htm)
    pub const MANI: u32 = tag!(b"mani"); // Manichaean
    pub const MEND: u32 = tag!(b"mend"); // Mende Kikakui
    pub const NBAT: u32 = tag!(b"nbat"); // Nabataean
    pub const NARB: u32 = tag!(b"narb"); // Old North Arabian
    pub const PALM: u32 = tag!(b"palm"); // Palmyrene
    pub const PHLP: u32 = tag!(b"phlp"); // Psalter Pahlavi
}

// Rudimentary script to direction mapping. Real implementation should implement the Unicode
// bidi algorithm (perhaps via a crate like yeslogic-unicode-bidi)
pub fn direction(script: u32) -> TextDirection {
    use rtl_tags as rtl;

    // Derived from https://github.com/foliojs/fontkit/blob/417af0c79c5664271a07a783574ec7fac7ebad0c/src/layout/Script.js#L195-L223
    // License: MIT Copyright (c) 2021 Devon Govett
    match script {
        | rtl::ARAB // Arabic
        | rtl::HEBR // Hebrew
        | rtl::SYRC // Syriac
        | rtl::THAA // Thaana
        | rtl::CPRT // Cypriot Syllabary
        | rtl::KHAR // Kharosthi
        | rtl::PHNX // Phoenician
        | rtl::NKO  // N'Ko
        | rtl::LYDI // Lydian
        | rtl::AVST // Avestan
        | rtl::ARMI // Imperial Aramaic
        | rtl::PHLI // Inscriptional Pahlavi
        | rtl::PRTI // Inscriptional Parthian
        | rtl::SARB // Old South Arabian
        | rtl::ORKH // Old Turkic, Orkhon Runic
        | rtl::SAMR // Samaritan
        | rtl::MAND // Mandaic, Mandaean
        | rtl::MERC // Meroitic Cursive
        | rtl::MERO // Meroitic Hieroglyphs
        | rtl::MANI // Manichaean
        | rtl::MEND // Mende Kikakui
        | rtl::NBAT // Nabataean
        | rtl::NARB // Old North Arabian
        | rtl::PALM // Palmyrene
        | rtl::PHLP => TextDirection::RightToLeft, // Psalter Pahlavi
        _ => TextDirection::LeftToRight,
    }
}
