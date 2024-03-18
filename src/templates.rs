/// Build default templates to generate documentation.
pub fn build_templates_registry() -> handlebars::Handlebars<'static> {
    let mut handlebars = handlebars::Handlebars::new();

    handlebars
        .register_template_string(
            "mdbook-module",
            include_str!("handlebars/mdbook/header.hbs"),
        )
        .expect("template is valid");
    handlebars
        .register_template_string(
            "docusaurus-module",
            include_str!("handlebars/docusaurus/header.hbs"),
        )
        .expect("template is valid");

    handlebars.set_prevent_indent(true);

    handlebars
}
