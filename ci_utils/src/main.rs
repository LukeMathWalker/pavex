use minijinja::{context, syntax::SyntaxConfig, Environment};

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
        ("lint_steps", "job_steps/lint.jinja"),
        ("build_clis_steps", "job_steps/build_clis.jinja"),
        ("examples_steps", "job_steps/examples.jinja"),
        (
            "build_tutorial_generator_steps",
            "job_steps/build_tutorial_generator.jinja",
        ),
        ("is_up_to_date_steps", "job_steps/is_up_to_date.jinja"),
        ("tests_steps", "job_steps/tests.jinja"),
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
    let output = env
        .get_template("ci")
        .unwrap()
        .render(context! {})
        .expect("Failed to and render template");
    println!("{output}");
}
