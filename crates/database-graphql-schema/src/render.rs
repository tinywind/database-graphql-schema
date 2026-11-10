#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SdlDocument {
    pub header: Option<String>,
    pub enums: Vec<SdlEnum>,
    pub types: Vec<SdlType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SdlEnum {
    pub name: String,
    pub values: Vec<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SdlType {
    pub name: String,
    pub fields: Vec<SdlField>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SdlField {
    pub name: String,
    pub graphql_type: String,
    pub non_null: bool,
    pub description: Option<String>,
}

pub(crate) fn render(document: &SdlDocument) -> String {
    let mut out = String::new();
    if let Some(header) = &document.header {
        render_header(&mut out, header);
        out.push_str("\n\n");
    }

    let mut declarations = Vec::with_capacity(document.enums.len() + document.types.len());
    declarations.extend(document.enums.iter().map(render_enum));
    declarations.extend(document.types.iter().map(render_type));
    out.push_str(&declarations.join("\n\n"));
    out.push('\n');
    out
}

fn render_header(out: &mut String, header: &str) {
    for (index, line) in header.trim_end().lines().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if line.is_empty() || line.starts_with('#') {
            out.push_str(line);
        } else {
            out.push_str("# ");
            out.push_str(line);
        }
    }
}

fn render_enum(value: &SdlEnum) -> String {
    let mut out = String::new();
    render_description(&mut out, value.description.as_deref(), "");
    out.push_str("enum ");
    out.push_str(&value.name);
    out.push_str(" {\n");
    for literal in &value.values {
        out.push_str("    ");
        out.push_str(literal);
        out.push('\n');
    }
    out.push('}');
    out
}

fn render_type(value: &SdlType) -> String {
    let mut out = String::new();
    render_description(&mut out, value.description.as_deref(), "");
    out.push_str("type ");
    out.push_str(&value.name);
    out.push_str(" {\n");
    for field in &value.fields {
        render_description(&mut out, field.description.as_deref(), "    ");
        out.push_str("    ");
        out.push_str(&field.name);
        out.push_str(": ");
        out.push_str(&field.graphql_type);
        if field.non_null {
            out.push('!');
        }
        out.push('\n');
    }
    out.push('}');
    out
}

fn render_description(out: &mut String, text: Option<&str>, indent: &str) {
    let Some(text) = text.filter(|text| !text.trim().is_empty()) else {
        return;
    };
    let text = text.trim_end();
    if text.contains('\n') || text.contains('"') {
        out.push_str(indent);
        out.push_str("\"\"\"\n");
        for line in text.lines() {
            out.push_str(indent);
            out.push_str(&line.replace("\"\"\"", "\\\"\"\""));
            out.push('\n');
        }
        out.push_str(indent);
        out.push_str("\"\"\"\n");
    } else {
        out.push_str(indent);
        out.push('"');
        out.push_str(&text.trim().replace('\\', "\\\\"));
        out.push_str("\"\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_is_deterministic_and_escapes_descriptions() {
        let document = SdlDocument {
            header: Some("Generated\n# fixed".to_owned()),
            enums: vec![SdlEnum {
                name: "Status".to_owned(),
                values: vec!["OPEN".to_owned(), "DONE".to_owned()],
                description: Some("States.".to_owned()),
            }],
            types: vec![SdlType {
                name: "Task".to_owned(),
                fields: vec![SdlField {
                    name: "note".to_owned(),
                    graphql_type: "String".to_owned(),
                    non_null: false,
                    description: Some("line one\n\"quoted\"".to_owned()),
                }],
                description: None,
            }],
        };
        let once = render(&document);
        let twice = render(&document);
        assert_eq!(once, twice);
        assert!(once.contains("# Generated\n# fixed"));
        assert!(once.contains("\"\"\"\n    line one\n    \"quoted\"\n    \"\"\""));
        assert!(once.ends_with('\n'));
    }
}
