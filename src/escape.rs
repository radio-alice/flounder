/// Copied from pulldown-cmark
/// https://github.com/raphlinus/pulldown-cmark/blob/e10b6801081799cab52dc3029a6de357b604f13e/src/escape.rs
/// Modified slightly
use std::str::from_utf8;

#[rustfmt::skip]
static HREF_SAFE: [u8; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0,
];

static HEX_CHARS: &[u8] = b"0123456789ABCDEF";
static AMP_ESCAPE: &str = "&amp;";
static SLASH_ESCAPE: &str = "&#x27;";

pub(crate) fn escape_href(w: &mut String, s: &str) {
    let bytes = s.as_bytes();
    let mut mark = 0;
    for i in 0..bytes.len() {
        let c = bytes[i];
        if c >= 0x80 || HREF_SAFE[c as usize] == 0 {
            // character needing escape

            // write partial substring up to mark
            if mark < i {
                w.push_str(&s[mark..i]);
            }
            match c {
                b'&' => {
                    w.push_str(AMP_ESCAPE);
                }
                b'\'' => {
                    w.push_str(SLASH_ESCAPE);
                }
                _ => {
                    let mut buf = [0u8; 3];
                    buf[0] = b'%';
                    buf[1] = HEX_CHARS[((c as usize) >> 4) & 0xF];
                    buf[2] = HEX_CHARS[(c as usize) & 0xF];
                    let escaped = from_utf8(&buf).unwrap();
                    w.push_str(escaped);
                }
            }
            mark = i + 1; // all escaped characters are ASCII
        }
    }
    w.push_str(&s[mark..])
}

const fn create_html_escape_table() -> [u8; 256] {
    let mut table = [0; 256];
    table[b'"' as usize] = 1;
    table[b'&' as usize] = 2;
    table[b'<' as usize] = 3;
    table[b'>' as usize] = 4;
    table
}

static HTML_ESCAPE_TABLE: [u8; 256] = create_html_escape_table();

static HTML_ESCAPES: [&'static str; 5] = ["", "&quot;", "&amp;", "&lt;", "&gt;"];

/// Writes the given string to the String, replacing special HTML bytes
/// (<, >, &, ") by escape sequences.
pub(crate) fn escape_html(w: &mut String, s: &str) {
    let bytes = s.as_bytes();
    let mut mark = 0;
    let mut i = 0;
    while i < s.len() {
        match bytes[i..]
            .iter()
            .position(|&c| HTML_ESCAPE_TABLE[c as usize] != 0)
        {
            Some(pos) => {
                i += pos;
            }
            None => break,
        }
        let c = bytes[i];
        let escape = HTML_ESCAPE_TABLE[c as usize];
        let escape_seq = HTML_ESCAPES[escape as usize];
        w.push_str(&s[mark..i]);
        w.push_str(escape_seq);
        i += 1;
        mark = i; // all escaped characters are ASCII
    }
    w.push_str(&s[mark..])
}
