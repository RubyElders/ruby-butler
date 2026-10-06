mod parse;
pub(crate) use parse::parse;

use kdl::{KdlDocument, KdlNode, KdlValue};
use std::io;
use toml::{Table, Value};

pub(crate) fn render(value: Value) -> io::Result<String> {
    let mut root = crate::sources::explicit_sources(value)
        .as_table()
        .unwrap()
        .clone();
    let sources = root.remove("sources");
    let dependencies = root.remove("dependencies");
    let groups = root.remove("groups");
    let mut output = document(&root)?;
    append_dependencies(&mut output, dependencies.as_ref())?;
    append_groups(&mut output, groups.as_ref())?;
    if let Some(Value::Array(sources)) = sources {
        for source in sources {
            let source = source.as_table().unwrap();
            let mut node = KdlNode::new("source");
            node.push(scalar(&source["url"])?);
            if let Some(default) = source.get("default") {
                node.push(("default", scalar(default)?));
            }
            let mut children = KdlDocument::new();
            append_dependencies(&mut children, source.get("dependencies"))?;
            append_groups(&mut children, source.get("groups"))?;
            node.set_children(children);
            output.nodes_mut().push(node);
        }
    }
    output.autoformat();
    Ok(format!(
        "{}{output}",
        crate::GENERATED_HEADER.replace("# ", "// ")
    ))
}

fn append_dependencies(document: &mut KdlDocument, dependencies: Option<&Value>) -> io::Result<()> {
    if let Some(Value::Table(dependencies)) = dependencies {
        for (name, value) in dependencies {
            if let Value::Array(declarations) = value {
                for value in declarations {
                    document.nodes_mut().push(dependency(name, value)?);
                }
            } else {
                document.nodes_mut().push(dependency(name, value)?);
            }
        }
    }
    Ok(())
}

fn append_groups(document: &mut KdlDocument, groups: Option<&Value>) -> io::Result<()> {
    if let Some(Value::Array(groups)) = groups {
        for group in groups {
            let mut node = KdlNode::new("group");
            for name in group["names"].as_array().unwrap() {
                node.push(scalar(name)?);
            }
            let mut children = KdlDocument::new();
            append_dependencies(&mut children, group.get("dependencies"))?;
            node.set_children(children);
            document.nodes_mut().push(node);
        }
    }
    Ok(())
}

fn dependency(name: &str, value: &Value) -> io::Result<KdlNode> {
    let mut node = KdlNode::new("gem");
    node.push(name);
    if let Value::String(version) = value {
        node.push(version.clone());
        return Ok(node);
    }
    let table = value.as_table().unwrap();
    if let Some(version) = table.get("version") {
        match version {
            Value::Array(versions) => {
                for version in versions {
                    node.push(scalar(version)?);
                }
            }
            _ => node.push(scalar(version)?),
        }
    }
    let mut children = KdlDocument::new();
    for (key, value) in table {
        if key == "version" {
            continue;
        }
        if value.is_table() || value.is_array() {
            children.nodes_mut().push(generic_node(key, value)?);
        } else {
            node.push((key.as_str(), scalar(value)?));
        }
    }
    if !children.nodes().is_empty() {
        node.set_children(children);
    }
    Ok(node)
}

fn document(table: &Table) -> io::Result<KdlDocument> {
    let mut document = KdlDocument::new();
    for (name, value) in table {
        document.nodes_mut().push(generic_node(name, value)?);
    }
    Ok(document)
}

fn generic_node(name: &str, value: &Value) -> io::Result<KdlNode> {
    let mut node = KdlNode::new(name);
    match value {
        Value::Table(table) => node.set_children(document(table)?),
        Value::Array(values) => {
            node.set_ty("array");
            if values
                .iter()
                .any(|value| value.is_table() || value.is_array())
            {
                let mut children = KdlDocument::new();
                for value in values {
                    children.nodes_mut().push(generic_node("item", value)?);
                }
                node.set_children(children);
            } else {
                for value in values {
                    node.push(scalar(value)?);
                }
            }
        }
        _ => node.push(scalar(value)?),
    }
    Ok(node)
}

fn scalar(value: &Value) -> io::Result<KdlValue> {
    Ok(match value {
        Value::String(value) => value.clone().into(),
        Value::Integer(value) => i128::from(*value).into(),
        Value::Float(value) => (*value).into(),
        Value::Boolean(value) => (*value).into(),
        Value::Datetime(value) => value.to_string().into(),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected a scalar value",
            ));
        }
    })
}
