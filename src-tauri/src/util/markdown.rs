use std::sync::LazyLock;

use anyhow::{Context, Result};
use pulldown_cmark::{html::push_html, Event, Options, Parser};
use regex::{Regex, RegexBuilder};

static AMMONIA: LazyLock<ammonia::Builder<'static>> = LazyLock::new(ammonia::Builder::default);

static EMOJI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r#":[\w\-+]+:"#)
        .unicode(false)
        .build()
        .unwrap()
});

pub fn render(input: &str, mut map_fn: impl FnMut(Event) -> Event) -> Result<String> {
    // parse markdown and render tokens into a buffer
    let tokens = Parser::new_ext(
        input,
        Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_GFM,
    )
    .map(|event| match event {
        Event::Text(s) => {
            let mut iter = EMOJI_REGEX.find_iter(&s);
            if let Some(mut first) = iter.next() {
                let mut buf = String::new();
                fn append_emoji(buf: &mut String, m: regex::Match<'_>) {
                    let s = m.as_str();
                    let code = &s[1..s.len() - 1];
                    if let Some(emoji) = emojis::get_by_shortcode(code) {
                        buf.push_str(emoji.as_str());
                    } else {
                        buf.push_str(s);
                    }
                }
                append_emoji(&mut buf, first);
                for m in iter {
                    buf.push_str(&s[first.end()..m.start()]);
                    append_emoji(&mut buf, first);
                    first = m;
                }
                buf.push_str(&s[first.end()..]);
                Event::Text(buf.into())
            } else {
                // TODO: GFM autolinks without `<` and `>`
                Event::Text(s)
            }
        }
        _ => event,
    })
    .map(map_fn);
    let mut buf = String::with_capacity(input.len());
    push_html(&mut buf, tokens);

    // sanitize the HTML, producing a parsed document
    let doc = AMMONIA.clean(&buf);

    // reuse the buffer
    let mut buf = buf.into_bytes();
    buf.clear();

    // write the document back to the existing buffer
    doc.write_to(&mut buf)?;

    String::from_utf8(buf).context("html5ever should only emit UTF-8")
}
