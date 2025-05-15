use std::sync::Arc;

use minijinja::{
    Environment, context,
    syntax::SyntaxConfig,
    value::{Enumerator, Object},
};

pub fn pavex_path(target: &str) -> String {
    match target {
        "linux" => "/home/runner/.cargo/bin/pavex".to_string(),
        "windows" => "C:\\Users\\runneradmin\\.cargo\\bin\\pavex.exe".to_string(),
        _ => "/Users/runner/.cargo/bin/pavex".to_string(),
    }
}

pub fn pavexc_path(target: &str) -> String {
    match target {
        "linux" => "/home/runner/.cargo/bin/pavexc".to_string(),
        "windows" => "C:\\Users\\runneradmin\\.cargo\\bin\\pavexc.exe".to_string(),
        _ => "/Users/runner/.cargo/bin/pavexc".to_string(),
    }
}

fn main() {
    let mut env = Environment::new();
    let syntax = SyntaxConfig::builder()
        .block_delimiters("<%", "%>")
        .variable_delimiters("<<", ">>")
        .build()
        .unwrap();
    env.set_syntax(syntax);
    let templates = [
        ("ci", "ci.jinja"),
        ("steps", "steps.jinja"),
        ("permissions", "permissions.jinja"),
        ("build_docs_steps", "job_steps/build_docs.jinja"),
        ("format_docs_steps", "job_steps/format_docs.jinja"),
        ("lint_steps", "job_steps/lint.jinja"),
        ("build_clis_steps", "job_steps/build_clis.jinja"),
        ("starter_steps", "job_steps/starter_example.jinja"),
        ("example_steps", "job_steps/example.jinja"),
        (
            "build_tutorial_generator_steps",
            "job_steps/build_tutorial_generator.jinja",
        ),
        ("is_up_to_date_steps", "job_steps/is_up_to_date.jinja"),
        ("tests_steps", "job_steps/tests.jinja"),
        ("macro_tests_steps", "job_steps/macro_tests.jinja"),
        ("ui_tests_steps", "job_steps/ui_tests.jinja"),
        ("setup_pavex", "setup_pavex.jinja"),
    ];
    let templates: Vec<_> = templates
        .into_iter()
        .map(|(name, path)| {
            let t =
                std::fs::read_to_string(format!("templates/{path}")).expect("Template not found");
            (name, t)
        })
        .collect();
    for (name, t) in &templates {
        env.add_template(name, t)
            .expect(&format!("{name} not found"));
    }
    env.add_function("pavex_path", pavex_path);
    env.add_function("pavexc_path", pavexc_path);
    let examples = {
        let entries = std::fs::read_dir("../examples").expect("Failed to find the examples folder");
        let mut examples = vec![];
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let type_ = entry.file_type().expect("Failed to get file type");
            if !type_.is_dir() {
                continue;
            }
            let name = entry.file_name().into_string().expect("Non UTF-8 dir name");
            if name == "starter" || name == ".cargo" {
                continue;
            }
            let services = entry.path().join(".github").join("services.yml");
            let services = std::fs::read_to_string(services).ok();
            let pre_steps = entry.path().join(".github").join("pre_steps.yml");
            let pre_steps = std::fs::read_to_string(pre_steps).ok();
            examples.push(minijinja::Value::from_object(Example {
                name,
                services,
                pre_steps,
            }))
        }
        examples
    };
    env.add_global("examples", examples);
    let output = env
        .get_template("ci")
        .unwrap()
        .render(context! {})
        .expect("Failed to and render template");
    println!("{output}");
}

#[derive(Debug, Clone)]
pub struct Example {
    pub name: String,
    pub services: Option<String>,
    pub pre_steps: Option<String>,
}

impl Object for Example {
    fn get_value(self: &Arc<Self>, key: &minijinja::Value) -> Option<minijinja::Value> {
        key.as_str().and_then(|key| match key {
            "name" => Some(minijinja::Value::from_safe_string(self.name.clone())),
            "services" => self
                .services
                .as_ref()
                .map(|s| minijinja::Value::from_safe_string(s.clone())),
            "pre_steps" => self
                .pre_steps
                .as_ref()
                .map(|s| minijinja::Value::from_safe_string(s.clone())),
            _ => None,
        })
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(&["name", "services", "pre_steps"])
    }
}
