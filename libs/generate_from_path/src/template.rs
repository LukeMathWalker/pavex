use crate::filenames::substitute_filename;
use crate::progressbar::spinner;
use crate::GenerateArgs;
use anyhow::Context;
use indicatif::{MultiProgress, ProgressBar};
use liquid::model::KString;
use liquid::Parser;
use liquid_core::{Object, Value};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub type LiquidObjectResource = Object;

/// create liquid object for the template, and pre-fill it with all known variables
pub fn create_liquid_object(args: &GenerateArgs) -> anyhow::Result<LiquidObjectResource> {
    let mut liquid_object = Object::new();
    liquid_object.insert(
        "project-name".into(),
        Value::Scalar(args.name.clone().into()),
    );
    Ok(liquid_object)
}

#[allow(clippy::too_many_arguments)]
pub fn walk_dir(
    project_dir: &Path,
    liquid_object: &mut LiquidObjectResource,
    liquid_engine: &Parser,
    mp: &mut MultiProgress,
    verbose: bool,
) -> anyhow::Result<()> {
    fn is_git_metadata(entry: &walkdir::DirEntry) -> bool {
        entry
            .path()
            .components()
            .any(|c| c == std::path::Component::Normal(".git".as_ref()))
    }

    let spinner_style = spinner();

    let mut files_with_errors = Vec::new();
    let files = WalkDir::new(project_dir)
        .sort_by_file_name()
        .contents_first(true)
        .into_iter()
        .filter_map(anyhow::Result::ok)
        .filter(|e| !is_git_metadata(e))
        .filter(|e| e.path() != project_dir)
        .collect::<Vec<_>>();
    let total = files.len().to_string();
    for (progress, entry) in files.into_iter().enumerate() {
        let pb = mp.add(ProgressBar::new(50));
        pb.set_style(spinner_style.clone());
        pb.set_prefix(format!(
            "[{:width$}/{}]",
            progress + 1,
            total,
            width = total.len()
        ));

        if !verbose {
            pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }

        let filename = entry.path();
        let relative_path = filename.strip_prefix(project_dir)?;
        let filename_display = relative_path.display();

        pb.set_message(format!("Processing: {filename_display}"));

        if entry.file_type().is_file() {
            match template_process_file(liquid_object, &liquid_engine, filename) {
                Err(e) => {
                    if verbose {
                        files_with_errors.push((filename.display().to_string(), e.clone()));
                    }
                }
                Ok(new_contents) => {
                    let new_filename = substitute_filename(filename, liquid_engine, liquid_object)
                        .with_context(|| {
                            format!("Error templating a filename `{}`", filename.display())
                        })?;
                    pb.inc(25);
                    let relative_path = new_filename.strip_prefix(project_dir)?;
                    let f = relative_path.display();
                    fs::create_dir_all(new_filename.parent().unwrap()).unwrap();
                    fs::write(new_filename.as_path(), new_contents).with_context(|| {
                        format!("Error writing rendered file `{}`", new_filename.display())
                    })?;
                    if filename != new_filename {
                        fs::remove_file(filename)?;
                    }
                    pb.inc(50);
                    pb.finish_with_message(format!("Done: {f}"));
                }
            }
        } else {
            let new_filename = substitute_filename(filename, liquid_engine, liquid_object)?;
            let relative_path = new_filename.strip_prefix(project_dir)?;
            let f = relative_path.display();
            pb.inc(50);
            if filename != new_filename {
                fs::remove_dir_all(filename)?;
            }
            pb.inc(50);
            pb.finish_with_message(format!("Done: {f}"));
        }
    }

    if !files_with_errors.is_empty() {
        print_files_with_errors_warning(files_with_errors);
    }

    Ok(())
}

fn template_process_file(
    context: &mut LiquidObjectResource,
    parser: &Parser,
    file: &Path,
) -> liquid_core::Result<String> {
    let content =
        fs::read_to_string(file).map_err(|e| liquid_core::Error::with_msg(e.to_string()))?;
    render_string_gracefully(context, parser, content.as_str())
}

pub fn render_string_gracefully(
    context: &mut LiquidObjectResource,
    parser: &Parser,
    content: &str,
) -> liquid_core::Result<String> {
    let template = parser.parse(content)?;

    let render_result = template.render(context);

    match render_result {
        ctx @ Ok(_) => ctx,
        Err(e) => {
            // handle it gracefully
            let msg = e.to_string();
            if msg.contains("requested variable") {
                // so, we miss a variable that is present in the file to render
                let requested_var =
                    regex::Regex::new(r"(?P<p>.*requested\svariable=)(?P<v>.*)").unwrap();
                let captures = requested_var.captures(msg.as_str()).unwrap();
                if let Some(Some(req_var)) = captures.iter().last() {
                    let missing_variable = KString::from(req_var.as_str().to_string());
                    // Substitute an empty string before retrying
                    let _ = context
                        .entry(missing_variable)
                        .or_insert_with(|| Value::scalar("".to_string()));
                    return render_string_gracefully(context, parser, content);
                }
            }

            // fallback: no rendering, keep things original
            Ok(content.to_string())
        }
    }
}

fn print_files_with_errors_warning(files_with_errors: Vec<(String, liquid_core::Error)>) {
    let mut msg = format!("Substitution skipped, found invalid syntax in\n");
    for file_error in files_with_errors {
        msg.push('\t');
        msg.push_str(&file_error.0);
        msg.push('\n');
    }
    tracing::warn!("{msg}");
}
