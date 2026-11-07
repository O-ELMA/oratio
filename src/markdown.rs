use pulldown_cmark::{html, Options, Parser};

pub fn to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown, options);
    let mut rendered = String::with_capacity(markdown.len() + markdown.len() / 2);
    html::push_html(&mut rendered, parser);

    let mut cleaner = ammonia::Builder::default();
    cleaner.add_generic_attributes(&["class"]);
    cleaner.clean(&rendered).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_markdown() {
        let html = to_html("# Title\n\nSome **bold** and `code`.\n\n```rust\nfn main() {}\n```");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<code"));
        assert!(html.contains("<pre>"));
    }

    #[test]
    fn renders_tables_and_lists() {
        let html = to_html("| a | b |\n|---|---|\n| 1 | 2 |\n\n- one\n- two\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("<li>one</li>"));
    }

    #[test]
    fn strips_scripts_and_event_handlers() {
        let html = to_html("<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n\n**ok**");
        assert!(!html.contains("script"));
        assert!(!html.contains("onerror"));
        assert!(html.contains("<strong>ok</strong>"));
    }
}
