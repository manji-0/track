pub(crate) fn render_markdown_with_links(content: &str) -> String {
    use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};
    use regex::Regex;
    use std::collections::{HashMap, HashSet};

    let url_regex =
        Regex::new(r"(?P<pre>^|[\s\(])(?P<url>https?://[^\s\)<>]+?)(?P<post>[.,;!?]*(?:[\s\)]|$))")
            .unwrap();

    let linkified = url_regex.replace_all(content, |caps: &regex::Captures| {
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

        format!("{}<{}>{}", pre, url, post)
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

    let allowed_tags: HashSet<&'static str> = [
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
    .collect();
    let allowed_attrs: HashSet<&'static str> = ["href", "title", "target"].into_iter().collect();
    let mut allowed_tag_attrs = HashMap::new();
    allowed_tag_attrs.insert("a", allowed_attrs);

    ammonia::Builder::default()
        .tags(allowed_tags)
        .tag_attributes(allowed_tag_attrs)
        .link_rel(Some("noopener noreferrer"))
        .clean(&html_output)
        .to_string()
}
