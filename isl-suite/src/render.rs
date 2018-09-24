use handlebars::Handlebars;

use isl::errors::*;

use super::SuiteResult;

pub fn render(res: &SuiteResult) -> Result<String> {
    let source = include_str!("render.html");

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    handlebars
        .render_template(source, &res)
        .map_err(|e| err_msg(format!("{:?}", e)))
}
