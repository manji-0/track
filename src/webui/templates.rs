//! MiniJinja template engine setup.

use minijinja::{path_loader, AutoEscape, Environment};
use std::path::PathBuf;
use std::sync::Arc;

/// Template engine wrapper
pub struct Templates {
    env: Environment<'static>,
}

impl Templates {
    /// Create new template engine with templates from the given directory
    #[allow(dead_code)]
    pub fn new(template_dir: PathBuf) -> Self {
        let mut env = Environment::new();
        env.set_loader(path_loader(template_dir));
        env.set_auto_escape_callback(|name| {
            if name.ends_with(".html") {
                AutoEscape::Html
            } else {
                AutoEscape::None
            }
        });

        Self { env }
    }

    /// Create template engine with embedded templates (for distribution)
    pub fn embedded() -> Self {
        let mut env = Environment::new();
        env.set_auto_escape_callback(|name| {
            if name.ends_with(".html") {
                AutoEscape::Html
            } else {
                AutoEscape::None
            }
        });

        // Embed templates at compile time
        env.add_template("base.html", include_str!("../../templates/base.html"))
            .expect("Failed to add base.html template");
        env.add_template("index.html", include_str!("../../templates/index.html"))
            .expect("Failed to add index.html template");
        env.add_template(
            "partials/todo_list.html",
            include_str!("../../templates/partials/todo_list.html"),
        )
        .expect("Failed to add todo_list.html template");
        env.add_template(
            "partials/scrap_list.html",
            include_str!("../../templates/partials/scrap_list.html"),
        )
        .expect("Failed to add scrap_list.html template");
        env.add_template(
            "partials/description.html",
            include_str!("../../templates/partials/description.html"),
        )
        .expect("Failed to add description.html template");
        env.add_template(
            "partials/ticket.html",
            include_str!("../../templates/partials/ticket.html"),
        )
        .expect("Failed to add ticket.html template");
        env.add_template(
            "partials/links.html",
            include_str!("../../templates/partials/links.html"),
        )
        .expect("Failed to add links.html template");
        env.add_template(
            "partials/repos.html",
            include_str!("../../templates/partials/repos.html"),
        )
        .expect("Failed to add repos.html template");
        env.add_template(
            "partials/calendar.html",
            include_str!("../../templates/partials/calendar.html"),
        )
        .expect("Failed to add calendar.html template");

        Self { env }
    }

    /// Render a template with the given context
    pub fn render<S: serde::Serialize>(&self, name: &str, ctx: S) -> anyhow::Result<String> {
        let tmpl = self.env.get_template(name)?;
        Ok(tmpl.render(ctx)?)
    }
}

/// Thread-safe template engine
pub type SharedTemplates = Arc<Templates>;
