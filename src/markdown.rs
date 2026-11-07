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
