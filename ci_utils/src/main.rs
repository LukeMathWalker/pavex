use minijinja::{context, syntax::SyntaxConfig, Environment};

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
        ("build_clis_steps", "job_steps/build_clis.jinja"),
        (
            "build_tutorial_generator_steps",
            "job_steps/build_tutorial_generator.jinja",
        ),
        ("is_up_to_date_steps", "job_steps/is_up_to_date.jinja"),
        ("tests_steps", "job_steps/tests.jinja"),
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
    let output = env
        .get_template("ci")
        .unwrap()
        .render(context! {})
        .expect("Failed to and render template");
    println!("{output}");
}
