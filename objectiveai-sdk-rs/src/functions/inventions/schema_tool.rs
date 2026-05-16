use std::collections::HashSet;

use super::tool::InventionTool;

mod schema_lookup {
    include!(concat!(env!("OUT_DIR"), "/schema_lookup.rs"));
}

pub fn schema_tools(schemas: &[&str]) -> Vec<InventionTool> {
    let mut seen = HashSet::new();
    let mut tools = Vec::new();
    let mut stack: Vec<&str> = schemas.iter().copied().collect();

    while let Some(name) = stack.pop() {
        if !seen.insert(name.to_string()) {
            continue;
        }

        let content = schema_lookup::schema_content(name)
            .unwrap_or_else(|| panic!("schema_tools: unknown schema {name:?}"));
        let tool_name = schema_lookup::tool_name(name).unwrap();
        let tool_desc = schema_lookup::tool_description(name).unwrap();

        tools.push(InventionTool::new_sync::<super::EmptyObjectJsonSchema>(
            tool_name,
            tool_desc,
            move |_| Ok(content.to_string()),
        ));

        for dep in schema_lookup::schema_refs(name) {
            if !seen.contains(*dep) {
                stack.push(dep);
            }
        }
    }

    tools
}
