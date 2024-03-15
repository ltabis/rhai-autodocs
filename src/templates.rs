/// Build default templates to generate documentation.
pub fn build_templates_registry() -> handlebars::Handlebars<'static> {
    let mut handlebars = handlebars::Handlebars::new();

    handlebars
        .register_template_string(
            "mdbook-header",
            include_str!("handlebars/mdbook/header.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string(
            "docusaurus-header",
            include_str!("handlebars/docusaurus/header.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string(
            "mdbook-function",
            include_str!("handlebars/mdbook/function.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string(
            "docusaurus-function",
            include_str!("handlebars/docusaurus/function.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string("mdbook-type", include_str!("handlebars/mdbook/type.hbs"))
        .expect("template is valid");
    handlebars
        .register_template_string(
            "docusaurus-type",
            include_str!("handlebars/docusaurus/type.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string("sections-rust", include_str!("handlebars/sections.hbs"))
        .expect("template is valid");
    handlebars
        .register_template_string(
            "mdbook-tab-sections",
            include_str!("handlebars/mdbook/sections.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string(
            "docusaurus-tab-sections",
            include_str!("handlebars/docusaurus/sections.hbs"),
        )
        .expect("template is valid");

    handlebars
}
