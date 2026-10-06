use crate::format::{compact, expand_dependencies, inherit_source};
use std::collections::BTreeMap;
use toml::{Table, Value};

pub(crate) fn explicit_sources(mut value: Value) -> Value {
    let root = value.as_table_mut().unwrap();
    let default = root
        .get_mut("bundle")
        .and_then(Value::as_table_mut)
        .and_then(|bundle| bundle.remove("source"));
    let mut sources = BTreeMap::<String, Table>::new();
    if let Some(Value::String(url)) = &default {
        sources.insert(
            url.clone(),
            Table::from_iter([
                ("url".into(), Value::String(url.clone())),
                ("default".into(), Value::Boolean(true)),
            ]),
        );
    }
    let dependencies = root
        .remove("dependencies")
        .unwrap()
        .as_table()
        .unwrap()
        .clone();
    let mut local = Table::new();
    for (name, value) in dependencies {
        let mut declarations = vec![value];
        while let Some(mut declaration) = declarations.pop() {
            let table = declaration.as_table_mut().unwrap();
            if let Some(Value::Array(variants)) = table.remove("variants") {
                declarations.extend(variants.into_iter().rev());
            }
            let source = table.get("source").cloned().or_else(|| {
                if table.contains_key("git") || table.contains_key("path") {
                    None
                } else {
                    default.clone()
                }
            });
            if source != default {
                table.remove("source");
            }
            let target = if let Some(Value::String(url)) = source {
                sources
                    .entry(url.clone())
                    .or_insert_with(|| Table::from_iter([("url".into(), Value::String(url))]))
            } else {
                &mut local
            };
            let groups = table
                .remove("groups")
                .unwrap_or_else(|| Value::Array(vec![Value::String("default".into())]));
            if groups
                .as_array()
                .is_some_and(|groups| groups == &[Value::String("default".into())])
            {
                insert_declaration(target, &name, declaration);
            } else {
                let entries = target
                    .entry("groups".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()))
                    .as_array_mut()
                    .unwrap();
                if let Some(existing) = entries
                    .iter_mut()
                    .find(|entry| entry.get("names") == Some(&groups))
                {
                    insert_declaration(existing.as_table_mut().unwrap(), &name, declaration);
                } else {
                    let mut group = Table::from_iter([("names".into(), groups)]);
                    insert_declaration(&mut group, &name, declaration);
                    entries.push(Value::Table(group));
                }
            }
        }
    }
    root.extend(local);
    if !sources.is_empty() {
        root.insert(
            "sources".into(),
            Value::Array(sources.into_values().map(Value::Table).collect()),
        );
    }
    value
}

fn insert_declaration(target: &mut Table, name: &str, declaration: Value) {
    let dependencies = target
        .entry("dependencies".to_string())
        .or_insert_with(|| Value::Table(Table::new()))
        .as_table_mut()
        .unwrap();
    let declaration = compact(declaration);
    match dependencies.get_mut(name) {
        None => {
            dependencies.insert(name.into(), declaration);
        }
        Some(Value::Array(values)) => values.push(declaration),
        Some(value) => *value = Value::Array(vec![value.clone(), declaration]),
    }
}

pub(crate) fn normalize_document(mut root: Table) -> Result<Table, String> {
    let mut dependencies = match root.remove("dependencies") {
        None => Table::new(),
        Some(Value::Table(value)) => expand_dependencies(value)?,
        Some(_) => return Err("dependencies must be a table".into()),
    };
    collect_groups(&mut root, None, &mut dependencies)?;
    if let Some(sources) = root.remove("sources") {
        for source in sources
            .as_array()
            .ok_or("sources must be an array of tables")?
        {
            let mut source = source.as_table().ok_or("source must be a table")?.clone();
            let url = source
                .remove("url")
                .and_then(|url| url.as_str().map(str::to_owned))
                .filter(|url| !url.is_empty())
                .ok_or("source requires a URL")?;
            let default = source
                .remove("default")
                .map(|value| value.as_bool().ok_or("source default must be a boolean"))
                .transpose()?
                .unwrap_or(false);
            if default {
                let bundle = root
                    .entry("bundle".to_string())
                    .or_insert_with(|| Value::Table(Table::new()))
                    .as_table_mut()
                    .ok_or("bundle must be a table")?;
                if bundle
                    .get("source")
                    .is_some_and(|value| value.as_str() != Some(&url))
                {
                    return Err("Conflicting default sources".into());
                }
                bundle.insert("source".into(), Value::String(url.clone()));
            }
            validate_source(&source, &url)?;
            let inherited = if default { None } else { Some(url.as_str()) };
            collect_dependencies(&mut source, inherited, None, &mut dependencies)?;
            collect_groups(&mut source, inherited, &mut dependencies)?;
            if !source.is_empty() {
                return Err("Unknown source field".into());
            }
        }
    }
    root.insert("dependencies".into(), Value::Table(dependencies));
    Ok(root)
}

fn collect_groups(
    root: &mut Table,
    source: Option<&str>,
    dependencies: &mut Table,
) -> Result<(), String> {
    if let Some(groups) = root.remove("groups") {
        for group in groups
            .as_array()
            .ok_or("groups must be an array of tables")?
        {
            let mut group = group.as_table().ok_or("group must be a table")?.clone();
            let names = group.remove("names").ok_or("group requires names")?;
            if !names.as_array().is_some_and(|names| {
                !names.is_empty()
                    && names
                        .iter()
                        .all(|name| name.as_str().is_some_and(|name| !name.is_empty()))
            }) {
                return Err("group names must be nonempty strings".into());
            }
            collect_dependencies(&mut group, source, Some(&names), dependencies)?;
            if !group.is_empty() {
                return Err("Unknown group field".into());
            }
        }
    }
    Ok(())
}

fn collect_dependencies(
    root: &mut Table,
    source: Option<&str>,
    groups: Option<&Value>,
    dependencies: &mut Table,
) -> Result<(), String> {
    if let Some(value) = root.remove("dependencies") {
        for (name, value) in value.as_table().ok_or("dependencies must be a table")? {
            let declarations = match value {
                Value::Array(values) => values.clone(),
                _ => vec![value.clone()],
            };
            if declarations.is_empty() {
                return Err("Empty dependency declaration list".into());
            }
            for mut value in declarations {
                if let Value::String(version) = value {
                    value = Value::Table(Table::from_iter([(
                        "version".into(),
                        Value::String(version),
                    )]));
                }
                if let Some(source) = source {
                    inherit_source(&mut value, source)?;
                }
                if let Some(groups) = groups {
                    let table = value
                        .as_table_mut()
                        .ok_or("dependency must be a string or table")?;
                    if table.get("groups").is_some_and(|value| value != groups) {
                        return Err("Dependency conflicts with its group block".into());
                    }
                    table.insert("groups".into(), groups.clone());
                }
                if let Some(current) = dependencies.get_mut(name) {
                    current
                        .as_table_mut()
                        .ok_or("dependency must be a table")?
                        .entry("variants".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()))
                        .as_array_mut()
                        .ok_or("variants must be an array")?
                        .push(value);
                } else {
                    dependencies.insert(name.clone(), value);
                }
            }
        }
    }
    Ok(())
}

fn validate_source(table: &Table, url: &str) -> Result<(), String> {
    if let Some(value) = table.get("dependencies") {
        for (_, dependency) in value.as_table().ok_or("dependencies must be a table")? {
            let values = match dependency {
                Value::Array(values) => values.clone(),
                value => vec![value.clone()],
            };
            for value in values {
                let mut value = if let Value::String(version) = value {
                    Value::Table(Table::from_iter([(
                        "version".into(),
                        Value::String(version),
                    )]))
                } else {
                    value
                };
                inherit_source(&mut value, url)?;
            }
        }
    }
    if let Some(groups) = table.get("groups") {
        for group in groups
            .as_array()
            .ok_or("groups must be an array of tables")?
        {
            validate_source(group.as_table().ok_or("group must be a table")?, url)?;
        }
    }
    Ok(())
}
