use regex::Regex;
use std::sync::OnceLock;

fn re(slot: &'static OnceLock<Regex>, pat: &'static str) -> &'static Regex {
    slot.get_or_init(|| Regex::new(pat).expect("wikitext regex"))
}

pub fn wikitext_to_text(src: &str) -> String {
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    static NOINCLUDE: OnceLock<Regex> = OnceLock::new();
    static REF: OnceLock<Regex> = OnceLock::new();
    static TEMPLATE: OnceLock<Regex> = OnceLock::new();
    static TABLE: OnceLock<Regex> = OnceLock::new();
    static BOLD: OnceLock<Regex> = OnceLock::new();
    static ITALIC: OnceLock<Regex> = OnceLock::new();
    static LINK: OnceLock<Regex> = OnceLock::new();
    static EXTLINK: OnceLock<Regex> = OnceLock::new();
    static FILE: OnceLock<Regex> = OnceLock::new();
    static NOWIKI: OnceLock<Regex> = OnceLock::new();
    static H6: OnceLock<Regex> = OnceLock::new();
    static H5: OnceLock<Regex> = OnceLock::new();
    static H4: OnceLock<Regex> = OnceLock::new();
    static H3: OnceLock<Regex> = OnceLock::new();
    static H2: OnceLock<Regex> = OnceLock::new();
    static H1: OnceLock<Regex> = OnceLock::new();

    let mut s = src.to_string();
    s = re(&COMMENT, r"(?s)<!--.*?-->").replace_all(&s, "").into_owned();
    s = re(&NOINCLUDE, r"(?is)<\s*/?\s*(noinclude|includeonly|onlyinclude)[^>]*>")
        .replace_all(&s, "")
        .into_owned();
    s = re(&NOWIKI, r"(?is)<nowiki>.*?</nowiki>")
        .replace_all(&s, "")
        .into_owned();
    s = re(&REF, r"(?is)<ref[^>]*>.*?</ref>|<ref[^/]*/>")
        .replace_all(&s, "")
        .into_owned();
    for _ in 0..8 {
        let next = re(&TEMPLATE, r"\{\{[^{}]*\}\}").replace_all(&s, "").into_owned();
        if next == s {
            break;
        }
        s = next;
    }
    s = re(&TABLE, r"(?m)^\s*\{\|.*$|^\s*\|\}.*$|^\s*\|-.*$|^\s*[|!].*$")
        .replace_all(&s, "")
        .into_owned();
    s = re(&FILE, r"\[\[(?:File|Image|Файл|Изображение):[^\]]*\]\]")
        .replace_all(&s, "")
        .into_owned();
    s = re(&LINK, r"\[\[([^\]|#]+)(?:\|([^\]]+))?\]\]")
        .replace_all(&s, |c: &regex::Captures| {
            c.get(2)
                .map(|m| m.as_str())
                .or_else(|| c.get(1).map(|m| m.as_str()))
                .unwrap_or("")
                .to_string()
        })
        .into_owned();
    s = re(&EXTLINK, r"\[(https?://[^\]\s]+)\s+([^\]]+)\]")
        .replace_all(&s, "$2")
        .into_owned();
    s = re(&EXTLINK, r"\[(https?://[^\]\s]+)\]")
        .replace_all(&s, "$1")
        .into_owned();
    s = re(&H6, r"(?m)^======\s*(.*?)\s*======\s*$")
        .replace_all(&s, "###### $1")
        .into_owned();
    s = re(&H5, r"(?m)^=====\s*(.*?)\s*=====\s*$")
        .replace_all(&s, "##### $1")
        .into_owned();
    s = re(&H4, r"(?m)^====\s*(.*?)\s*====\s*$")
        .replace_all(&s, "#### $1")
        .into_owned();
    s = re(&H3, r"(?m)^===\s*(.*?)\s*===\s*$")
        .replace_all(&s, "### $1")
        .into_owned();
    s = re(&H2, r"(?m)^==\s*(.*?)\s*==\s*$")
        .replace_all(&s, "## $1")
        .into_owned();
    s = re(&H1, r"(?m)^=\s*(.*?)\s*=\s*$")
        .replace_all(&s, "# $1")
        .into_owned();
    s = re(&BOLD, r"'''([^']+)'''").replace_all(&s, "**$1**").into_owned();
    s = re(&ITALIC, r"''([^']+)''").replace_all(&s, "*$1*").into_owned();
    s = s.replace("&nbsp;", " ");
    collapse_blank_lines(&s)
}

pub fn html_to_text(html: &str) -> String {
    static SCRIPT: OnceLock<Regex> = OnceLock::new();
    static STYLE: OnceLock<Regex> = OnceLock::new();
    static BR: OnceLock<Regex> = OnceLock::new();
    static P: OnceLock<Regex> = OnceLock::new();
    static LI: OnceLock<Regex> = OnceLock::new();
    static TAG: OnceLock<Regex> = OnceLock::new();
    static ENT: OnceLock<Regex> = OnceLock::new();
    static HH1: OnceLock<Regex> = OnceLock::new();
    static HH2: OnceLock<Regex> = OnceLock::new();
    static HH3: OnceLock<Regex> = OnceLock::new();
    static HH4: OnceLock<Regex> = OnceLock::new();
    static HH5: OnceLock<Regex> = OnceLock::new();
    static HH6: OnceLock<Regex> = OnceLock::new();

    let mut s = html.to_string();
    s = re(&SCRIPT, r"(?is)<script[^>]*>.*?</script>")
        .replace_all(&s, " ")
        .into_owned();
    s = re(&STYLE, r"(?is)<style[^>]*>.*?</style>")
        .replace_all(&s, " ")
        .into_owned();
    s = re(&BR, r"(?i)<br\s*/?>").replace_all(&s, "\n").into_owned();
    s = re(&P, r"(?i)</p\s*>").replace_all(&s, "\n\n").into_owned();
    s = re(&LI, r"(?i)<li[^>]*>").replace_all(&s, "\n- ").into_owned();
    for (slot, pat, hashes) in [
        (&HH1, r"(?is)<h1[^>]*>(.*?)</h1>", "#"),
        (&HH2, r"(?is)<h2[^>]*>(.*?)</h2>", "##"),
        (&HH3, r"(?is)<h3[^>]*>(.*?)</h3>", "###"),
        (&HH4, r"(?is)<h4[^>]*>(.*?)</h4>", "####"),
        (&HH5, r"(?is)<h5[^>]*>(.*?)</h5>", "#####"),
        (&HH6, r"(?is)<h6[^>]*>(.*?)</h6>", "######"),
    ] {
        s = re(slot, pat)
            .replace_all(&s, |c: &regex::Captures| {
                let title = strip_tags(c.get(1).map(|m| m.as_str()).unwrap_or(""));
                format!("\n{hashes} {}\n", title.trim())
            })
            .into_owned();
    }
    s = re(&TAG, r"<[^>]+>").replace_all(&s, " ").into_owned();
    s = re(&ENT, r"&nbsp;").replace_all(&s, " ").into_owned();
    s = s.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">");
    s = s.replace("&quot;", "\"").replace("&#39;", "'");
    collapse_blank_lines(&s)
}

fn strip_tags(s: &str) -> String {
    static TAG: OnceLock<Regex> = OnceLock::new();
    re(&TAG, r"<[^>]+>").replace_all(s, "").into_owned()
}

fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::new();
    let mut blank = 0usize;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            blank += 1;
            if blank <= 2 {
                out.push('\n');
            }
        } else {
            blank = 0;
            out.push_str(t);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

pub fn extract_content_root(html: &str) -> Option<String> {
    static MW: OnceLock<Regex> = OnceLock::new();
    static CONTENT: OnceLock<Regex> = OnceLock::new();
    static MAIN: OnceLock<Regex> = OnceLock::new();
    static ARTICLE: OnceLock<Regex> = OnceLock::new();
    let patterns = [
        (
            &MW,
            r#"(?is)<div[^>]+id=["']mw-content-text["'][^>]*>(.*?)</div>"#,
        ),
        (
            &CONTENT,
            r#"(?is)<div[^>]+id=["']content["'][^>]*>(.*?)</div>"#,
        ),
        (&MAIN, r"(?is)<main[^>]*>(.*?)</main>"),
        (&ARTICLE, r"(?is)<article[^>]*>(.*?)</article>"),
    ];
    for (slot, pat) in patterns {
        if let Some(c) = re(slot, pat).captures(html)
            && let Some(m) = c.get(1)
        {
            let inner = m.as_str().trim();
            if inner.len() >= 20 {
                return Some(inner.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_headings_and_links() {
        let t = wikitext_to_text("== Login ==\nSee [[Dashboard|home]] and [[Help]].\n");
        assert!(t.contains("# Login") || t.contains("## Login"));
        assert!(t.contains("home"));
        assert!(t.contains("Help"));
    }

    #[test]
    fn extracts_mw_content() {
        let html = r#"<html><body><div id="mw-content-text"><p>Hello wiki body text here</p></div><div id="footer">x</div></body></html>"#;
        let inner = extract_content_root(html).unwrap();
        assert!(inner.contains("Hello wiki"));
        let text = html_to_text(&inner);
        assert!(text.contains("Hello wiki"));
    }
}
