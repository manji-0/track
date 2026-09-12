use pulldown_cmark::{Event, Parser, Tag, TagEnd, html};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<pre>^|[\s\(])(?P<url>https?://[^\s\)<>]+?)(?P<post>[.,;!?]*(?:[\s\)]|$))")
        .expect("url autolink regex is valid")
});

static ALLOWED_TAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a",
        "p",
        "ul",
        "ol",
        "li",
        "strong",
        "em",
        "code",
        "pre",
        "blockquote",
        "br",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
    ]
    .into_iter()
    .collect()
});

static ALLOWED_TAG_ATTRS: LazyLock<HashMap<&'static str, HashSet<&'static str>>> =
    LazyLock::new(|| {
        let allowed_attrs: HashSet<&'static str> =
            ["href", "title", "target"].into_iter().collect();
        let mut allowed_tag_attrs = HashMap::new();
        allowed_tag_attrs.insert("a", allowed_attrs);
        allowed_tag_attrs
    });

pub(crate) fn render_markdown_with_links(content: &str) -> String {
    let linkified = URL_REGEX.replace_all(content, |caps: &regex::Captures| {
        let pre = &caps["pre"];
        let url = &caps["url"];
        let post = &caps["post"];

        let cap_start = caps.get(0).unwrap().start();
        if cap_start >= 2 {
            let before = &content[..cap_start];
            if before.ends_with("](") {
                return caps.get(0).unwrap().as_str().to_string();
            }
        }

        format!("{pre}<{url}>{post}")
    });

    let parser = Parser::new(linkified.as_ref());
    let sanitized = parser.map(|event| match event {
        Event::Html(html) | Event::InlineHtml(html) => {
            let escaped = html_escape::encode_safe(&html).into_owned();
            Event::Text(escaped.into())
        }
        _ => event,
    });

    let parser_with_target = sanitized.map(|event| match event {
        Event::Start(Tag::Link {
            link_type: _,
            dest_url,
            title,
            id: _,
        }) => Event::Html(
            format!(
                r#"<a href="{}" target="_blank" rel="noopener noreferrer"{}>"#,
                html_escape::encode_double_quoted_attribute(&dest_url),
                if !title.is_empty() {
                    format!(
                        r#" title="{}""#,
                        html_escape::encode_double_quoted_attribute(&title)
                    )
                } else {
                    String::new()
                }
            )
            .into(),
        ),
        Event::End(TagEnd::Link) => Event::Html("</a>".into()),
        _ => event,
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser_with_target);

    let cleaned = ammonia::Builder::default()
        .tags(ALLOWED_TAGS.clone())
        .tag_attributes(ALLOWED_TAG_ATTRS.clone())
        .link_rel(Some("noopener noreferrer"))
        .clean(&html_output);
    cleaned.to_string()
}
