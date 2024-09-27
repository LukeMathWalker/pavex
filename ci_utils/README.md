# Why?

GitHub Actions is quite inflexible when it comes to reusing pieces of a workflow.  
They don't support YAML anchors to deduplicate common steps and their reusable workflows are extremely limited 
(e.g. you can't pass the `github` context down as a parameter).

Rather than implementing an extremely complex workaround *inside* GitHub Actions, we wrap around it with a simple
Rust script that generates the workflow file for us.  
This keeps the workflow file readable and maintainable.
