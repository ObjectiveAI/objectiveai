pub fn readme(
    name: &str,
    description: &str,
    sub_functions: Vec<String>,
) -> String {
    let mut out = format!("# {name}\n\n{description}\n");

    if !sub_functions.is_empty() {
        out.push_str("\n## Sub-Functions\n\n");
        for sf in &sub_functions {
            out.push_str(&format!("- {sf}\n"));
        }
    }

    out
}
