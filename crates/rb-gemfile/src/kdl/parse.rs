use kdl::{KdlDocument, KdlNode, KdlValue};
use std::io;
use toml::{Table, Value};

pub(crate) fn parse(input: &str) -> io::Result<Value> {
    let document: KdlDocument = input.parse().map_err(invalid)?;
    let mut root = Table::new();
    let mut dependencies = Table::new();
    parse_nodes(&document, None, &[], &mut root, &mut dependencies)?;
    root.insert("dependencies".into(), Value::Table(dependencies));
    Ok(Value::Table(root))
}

fn parse_nodes(
    document: &KdlDocument,
    source: Option<(&str, bool)>,
    groups: &[String],
    root: &mut Table,
    dependencies: &mut Table,
) -> io::Result<()> {
    for node in document.nodes() {
        match node.name().value() {
            "source" => {
                let url = node
                    .get(0)
                    .and_then(KdlValue::as_string)
                    .filter(|url| !url.is_empty())
                    .ok_or_else(|| invalid("source requires URL"))?;
                let default = match node.get("default") {
                    None => false,
                    Some(value) => value
                        .as_bool()
                        .ok_or_else(|| invalid("source default must be boolean"))?,
                };
                if node
                    .entries()
                    .iter()
                    .any(|entry| entry.name().is_some_and(|name| name.value() != "default"))
                    || node
                        .entries()
                        .iter()
                        .filter(|entry| entry.name().is_none())
                        .count()
                        != 1
                {
                    return Err(invalid("Unknown source entry"));
                }
                if default {
                    let bundle = root
                        .entry("bundle".to_string())
                        .or_insert_with(|| Value::Table(Table::new()))
                        .as_table_mut()
                        .ok_or_else(|| invalid("bundle must be a table"))?;
                    if bundle
                        .get("source")
                        .is_some_and(|value| value.as_str() != Some(url))
                    {
                        return Err(invalid("Conflicting default sources"));
                    }
                    bundle.insert("source".into(), Value::String(url.into()));
                }
                if let Some(children) = node.children() {
                    parse_nodes(children, Some((url, default)), groups, root, dependencies)?;
                }
            }
            "group" => {
                let mut inherited = groups.to_vec();
                for entry in node.entries() {
                    if entry.name().is_some() {
                        return Err(invalid("group expects names"));
                    }
                    let name = entry
                        .value()
                        .as_string()
                        .filter(|name| !name.is_empty())
                        .ok_or_else(|| invalid("group names must be nonempty strings"))?;
                    if !inherited.iter().any(|value| value == name) {
                        inherited.push(name.into());
                    }
                }
                if node.entries().is_empty() {
                    return Err(invalid("group requires names"));
                }
                if let Some(children) = node.children() {
                    parse_nodes(children, source, &inherited, root, dependencies)?;
                }
            }
            "gem" => {
                let name = node
                    .get(0)
                    .and_then(KdlValue::as_string)
                    .ok_or_else(|| invalid("gem requires a name"))?;
                let mut table = Table::new();
                let mut versions = Vec::new();
                for entry in node
                    .entries()
                    .iter()
                    .filter(|entry| entry.name().is_none())
                    .skip(1)
                {
                    versions.push(kdl_scalar(entry.value())?);
                }
                if !versions.is_empty() {
                    table.insert("version".into(), Value::Array(versions));
                }
                for entry in node.entries().iter().filter(|entry| entry.name().is_some()) {
                    let key = entry.name().unwrap().value();
                    if table
                        .insert(key.into(), kdl_scalar(entry.value())?)
                        .is_some()
                    {
                        return Err(invalid("Duplicate gem option"));
                    }
                }
                if let Some(children) = node.children() {
                    for child in children.nodes() {
                        if table
                            .insert(child.name().value().into(), generic_value(child)?)
                            .is_some()
                        {
                            return Err(invalid("Duplicate gem option"));
                        }
                    }
                }
                if let Some((url, default)) = source {
                    if table.contains_key("git")
                        || table.contains_key("path")
                        || table
                            .get("source")
                            .is_some_and(|value| value.as_str() != Some(url))
                    {
                        return Err(invalid("Dependency conflicts with its source block"));
                    }
                    if !default {
                        table.insert("source".into(), Value::String(url.into()));
                    }
                }
                if !groups.is_empty() {
                    let value = Value::Array(groups.iter().cloned().map(Value::String).collect());
                    if table.get("groups").is_some_and(|groups| groups != &value) {
                        return Err(invalid("Dependency conflicts with its group block"));
                    }
                    table.insert("groups".into(), value);
                }
                if let Some(current) = dependencies.get_mut(name) {
                    current
                        .as_table_mut()
                        .ok_or_else(|| invalid("dependency must be a table"))?
                        .entry("variants".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()))
                        .as_array_mut()
                        .ok_or_else(|| invalid("variants must be an array"))?
                        .push(Value::Table(table));
                } else {
                    dependencies.insert(name.into(), Value::Table(table));
                }
            }
            _ => {
                if source.is_some() || !groups.is_empty() {
                    return Err(invalid("Unknown dependency block"));
                }
                if root
                    .insert(node.name().value().into(), generic_value(node)?)
                    .is_some()
                {
                    return Err(invalid("Duplicate metadata node"));
                }
            }
        }
    }
    Ok(())
}

fn generic_value(node: &KdlNode) -> io::Result<Value> {
    if let Some(children) = node.children() {
        if !node.entries().is_empty() {
            return Err(invalid("Metadata cannot mix entries and children"));
        }
        if node.ty().is_some_and(|ty| ty.value() == "array")
            && children
                .nodes()
                .iter()
                .all(|node| node.name().value() == "item")
            && !children.nodes().is_empty()
        {
            return children
                .nodes()
                .iter()
                .map(generic_value)
                .collect::<io::Result<Vec<_>>>()
                .map(Value::Array);
        }
        let mut table = Table::new();
        for child in children.nodes() {
            if table
                .insert(child.name().value().into(), generic_value(child)?)
                .is_some()
            {
                return Err(invalid("Duplicate metadata key"));
            }
        }
        return Ok(Value::Table(table));
    }
    if node.entries().iter().any(|entry| entry.name().is_some()) {
        return Err(invalid("Metadata expects positional values"));
    }
    let values = node
        .entries()
        .iter()
        .map(|entry| kdl_scalar(entry.value()))
        .collect::<io::Result<Vec<_>>>()?;
    if values.len() == 1
        && node.ty().is_none()
        && !matches!(
            node.name().value(),
            "groups"
                | "platforms"
                | "optional_groups"
                | "required_ruby_version"
                | "required_rubygems_version"
        )
    {
        Ok(values.into_iter().next().unwrap())
    } else {
        Ok(Value::Array(values))
    }
}

fn kdl_scalar(value: &KdlValue) -> io::Result<Value> {
    if let Some(value) = value.as_string() {
        Ok(Value::String(value.into()))
    } else if let Some(value) = value.as_bool() {
        Ok(Value::Boolean(value))
    } else if let Some(value) = value.as_integer() {
        Ok(Value::Integer(i64::try_from(value).map_err(invalid)?))
    } else if let Some(value) = value.as_float() {
        Ok(Value::Float(value))
    } else {
        Err(invalid("Unsupported KDL scalar"))
    }
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
