//! An implementation of gmi -> HTML conversion, based on
//! the [text/gemini](https://gemini.circumlunar.space/docs/specification.html) spec v0.14.2

use crate::escape;

use escape::{escape_href, escape_html};
use url::{ParseError, Url};

// Some configuration
static ALLOWED_SCHEMES: &[&str] = &["https", "http", "gemini", "gopher", "mailto"];

// All 4 characters for efficiency
// set to [] for none 
static INLINE_IMAGE_EXTENSIONS: &[&str] = &[".jpg", "jpeg", ".png", ".gif", ".ico", ".svg", "webp"];

// All 4 characters for efficiency
// set to [] for none 
static INLINE_AUDIO_EXTENSIONS: &[&str] = &[".mp3"];

static PROXY_URL: &str = "https://portal.mozz.us/gemini/";


/// Convert Gemini text to Flounder-styled HTML.
pub fn gemtext_to_html(input_text: &str) -> String {
    // This function sometimes priorities performance over readability
    let proxy_url: Url = PROXY_URL.parse().unwrap(); // TODO move to static scope
    let mut output = String::new();
    let mut is_pre = false;
    let mut is_list = false;
    for line in input_text.lines() {
        // See 5.4.3 "Preformatting toggle lines"
        if line.starts_with("```") {
            is_pre = !is_pre;
            if is_pre {
                if line.len() > 3 {
                    // This is marginally faster than using format!, albeit a bit uglier
                    output.push_str("<pre alt=\"");
                    escape_html(&mut output, &line[3..]);
                    output.push_str("\">\n");
                } else {
                    output.push_str("<pre>\n");
                }
            } else {
                output.push_str("</pre>\n")
            }
            continue;
        }
        if is_pre {
            escape_html(&mut output, line);
            output.push('\n');
            continue;
        }
        // See 5.5.2 "Unordered list items"
        if line.starts_with("* ") {
            if !is_list {
                output.push_str("<ul>\n");
                is_list = true;
            }
            output.push_str("<li>");
            escape_html(&mut output, &line[2..].trim());
            output.push_str("</li>\n");
            continue;
        } else {
            if is_list {
                output.push_str("</ul>\n");
            }
            is_list = false;
        }
        // 5.5.1 heading lines
        if line.starts_with("#") {
            let mut count = 0;
            for ch in line.chars() {
                if ch == '#' {
                    count += 1;
                    // Limit to 3 headers.
                    if count == 3 {
                        break;
                    }
                }
            }
            // String allocation for readability
            output.push_str(&format!("<h{}>", count));
            escape_html(&mut output, &line[count..].trim());
            output.push_str(&format!("</h{}>\n", count));
        // 5.5.3 Quote lines
        } else if line.starts_with(">") {
            output.push_str("<q>");
            escape_html(&mut output, &line[1..]);
            output.push_str("</q><br>\n");
        } else if line.starts_with("=>") {
            let mut i = line[2..].split_whitespace();
            let first: &str = i.next().unwrap_or("");
            // inefficient
            let second: String = i.collect::<Vec<&str>>().join(" ");
            // This is much slower than surrounding code
            // TODO consider blacklist
            let parsed = Url::parse(first);
            let mut is_image = false;
            let mut is_audio = false;
            if parsed == Err(ParseError::RelativeUrlWithoutBase) {
                let extension: &str = &first[first.len()-4..first.len()].to_ascii_lowercase();
                if INLINE_IMAGE_EXTENSIONS.contains(&extension) {
                    output.push_str("<img src=\"");
                    is_image = true;
                } else if INLINE_AUDIO_EXTENSIONS.contains(&extension) {
                    output.push_str("<audio controls src=\""); // TODO audio
                    is_audio = true;
                } else {
                    output.push_str("<a href=\"");
                }
                let relative_url = String::new();
                escape_href(&mut output, first);
                output.push_str(&relative_url);
            } else {
                output.push_str("<a href=\"");
            }
            if let Ok(p) = parsed {
                if ALLOWED_SCHEMES.contains(&p.scheme()) {
                    if p.scheme() == "gemini" {
                        // TODO FIX
                            // Never fail, just use blank string if cant parse
                            let join = |a: &Url, b: Url| ->  Result<String, Box<dyn std::error::Error>> {
                                Ok(a.join(b.host_str().ok_or("err")?)?.join(b.path())?.as_str().to_string())
                            };
                            let proxied = join(&proxy_url, p).unwrap_or("".to_string()); // Dont fail
                            output.push_str(&proxied);
                    } else {
                        output.push_str(p.as_str());
                    }
                }
            }
            let link_text = match second.as_str() {
                "" => first,
                t => t,
            };
            if is_image {
                output.push_str("\" alt=\"");
                escape_html(&mut output, link_text);
                output.push_str("\">");
            } else if is_audio {
                output.push_str("\" alt=\"");
                escape_html(&mut output, link_text);
                output.push_str("\">");
                output.push_str("</audio>")
            } else {
                output.push_str("\">");
                escape_html(&mut output, link_text);
                output.push_str("</a>");
            }
            output.push_str("<br>\n");
        } else {
            escape_html(&mut output, line);
            output.push_str("<br>\n");
        }
    }
    // Check outstanding tags that need to be closed
    if is_list {
        output.push_str("</ul>");
    }
    if is_pre {
        output.push_str("</pre>")
    }
    return output;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic() {
        assert_eq!(
            gemtext_to_html("hello world"),
            "hello world<br>\n"
        )
    }

    #[test]
    fn test_unsafe_html() {
        assert_eq!(
            gemtext_to_html("<b>hacked</b>"),
            "&lt;b&gt;hacked&lt;/b&gt;<br>\n"
        );
        // TODO add more tests
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(
            gemtext_to_html("\n\n\n"),
            "<br>\n<br>\n<br>\n"
        )
    }

    #[test]
    fn test_list() {
        assert_eq!(
            gemtext_to_html("hi\n* cool\n* vibes\nok"),
            "hi<br>\n<ul>\n<li>cool</li>\n<li>vibes</li>\n</ul>\nok<br>\n"
            )
    }

    #[test]
    fn test_quote() {
        assert_eq!(
            gemtext_to_html("> stay cool\n-coolguy"),
            "<q> stay cool</q><br>\n-coolguy<br>\n"
            )
    }
    #[test]
    fn test_headers() {
        assert_eq!(
            gemtext_to_html("#header"),
            "<h1>header</h1>\n"
            );
        assert_eq!(
            gemtext_to_html("##header"),
            "<h2>header</h2>\n"
            );
        assert_eq!(
            gemtext_to_html("### header"),
            "<h3>header</h3>\n"
            );
        assert_eq!(
            gemtext_to_html("####header"),
            "<h3>#header</h3>\n"
            );
    }

    #[test]
    fn test_pre() {
        assert_eq!(
            gemtext_to_html("```\nhello world\n```"),
            "<pre>\nhello world\n</pre>\n"
            );
    }

    #[test]
    fn test_pre_alt() {
        assert_eq!(
            gemtext_to_html("```alt\"\nhello world\n```"),
            "<pre alt=\"alt&quot;\">\nhello world\n</pre>\n"
            );
    }

    #[test]
    fn test_hyperlink() {
        assert_eq!(
            // TODO resolve trailing slash issue
            gemtext_to_html("=> https://google.com"),
            "<a href=\"https://google.com/\">https://google.com</a><br>\n"
            )
    }

    #[test]
    fn test_replace_image() {
        assert_eq!(
            gemtext_to_html("=> something.jpg cool pic"),
            "<img src=\"something.jpg\" alt=\"cool pic\"><br>\n"
            )
    }
    #[test]
    fn test_replace_audio() {
        assert_eq!(
            gemtext_to_html("=> something.mp3 cool audio"),
            "<audio controls src=\"something.mp3\" alt=\"cool audio\"></audio><br>\n"
            )
    }
}
