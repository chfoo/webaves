use fluent_templates::Loader;

fluent_templates::static_loader! {
    pub static LOCALES = {
        locales: "locales",
        fallback_language: "en-US",
    };
}

pub fn static_text(id: &str) -> &'static str {
    let text = LOCALES.lookup(&unic_langid::langid!("en-US"), id);

    Box::leak(text.into_boxed_str())
}
