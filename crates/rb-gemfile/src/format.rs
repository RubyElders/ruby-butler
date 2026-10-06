use crate::Dependency;
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use toml::{Table, Value};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExportFormat {
    #[default]
    Toml,
    Kdl,
}

pub(crate) fn is_source(key: &str) -> bool {
    key.contains("://")
}

pub(crate) fn expand_dependencies(dependencies: Table) -> Result<Table, String> {
    let mut expanded = Table::new();
    for (key, value) in dependencies {
        let entries = if is_source(&key) {
            value
                .as_table()
                .ok_or_else(|| "Source dependencies must be a table".to_string())?
                .clone()
        } else {
            Table::from_iter([(key.clone(), value)])
        };
        for (name, mut value) in entries {
            if let Value::Array(declarations) = value {
                let mut declarations = declarations.into_iter();
                value = declarations
                    .next()
                    .ok_or("Empty dependency declaration list")?;
                if let Value::String(version) = value {
                    value = Value::Table(Table::from_iter([(
                        "version".into(),
                        Value::String(version),
                    )]));
                }
                let variants = declarations
                    .map(|value| match value {
                        Value::String(version) => Value::Table(Table::from_iter([(
                            "version".into(),
                            Value::String(version),
                        )])),
                        value => value,
                    })
                    .collect::<Vec<_>>();
                if !variants.is_empty() {
                    value
                        .as_table_mut()
                        .ok_or("Dependency must be a string or table")?
                        .insert("variants".into(), Value::Array(variants));
                }
            }
            if let Value::String(version) = value {
                value = Value::Table(Table::from_iter([(
                    "version".into(),
                    Value::String(version),
                )]));
            }
            if is_source(&key) {
                inherit_source(&mut value, &key)?;
            }
            if expanded.insert(name.clone(), value).is_some() {
                return Err(format!("Duplicate dependency {name} across source blocks"));
            }
        }
    }
    Ok(expanded)
}

pub(crate) fn inherit_source(value: &mut Value, source: &str) -> Result<(), String> {
    let table = value
        .as_table_mut()
        .ok_or_else(|| "Dependency must be a string or table".to_string())?;
    if table.contains_key("git")
        || table.contains_key("path")
        || table
            .get("source")
            .is_some_and(|value| value.as_str() != Some(source))
    {
        return Err("Dependency conflicts with its source block".into());
    }
    table.insert("source".into(), Value::String(source.into()));
    if let Some(Value::Array(variants)) = table.get_mut("variants") {
        for variant in variants {
            inherit_source(variant, source)?;
        }
    }
    Ok(())
}

pub(crate) fn deserialize_dependencies<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Dependency>, D::Error> {
    let dependencies = Table::deserialize(deserializer)?;
    let expanded = expand_dependencies(dependencies).map_err(serde::de::Error::custom)?;
    expanded
        .into_iter()
        .map(|(name, value)| {
            let dependency: Dependency = value.try_into().map_err(serde::de::Error::custom)?;
            dependency.validate().map_err(serde::de::Error::custom)?;
            Ok((name, dependency))
        })
        .collect()
}

pub(crate) fn compact(mut value: Value) -> Value {
    let table = value.as_table_mut().unwrap();
    if table
        .get("groups")
        .and_then(Value::as_array)
        .is_some_and(|groups| groups == &[Value::String("default".into())])
    {
        table.remove("groups");
    }
    if let Some(Value::Array(variants)) = table.get_mut("variants") {
        for variant in variants {
            *variant = compact(variant.clone());
            if let Value::String(version) = variant {
                *variant = Value::Table(Table::from_iter([(
                    "version".into(),
                    Value::String(version.clone()),
                )]));
            }
        }
    }
    if table.len() == 1 && table.contains_key("version") && table["version"].is_str() {
        return table.remove("version").unwrap();
    }
    if table.get("version").and_then(Value::as_str) == Some(">= 0") {
        table.remove("version");
    }
    value
}

pub(crate) fn toml(value: Value) -> Result<String, toml::ser::Error> {
    let value = crate::sources::explicit_sources(value);
    let mut document = toml_edit::ser::to_document(&value).map_err(serde::ser::Error::custom)?;
    compact_tables(document.as_table_mut());
    Ok(format!("{}{document}", crate::GENERATED_HEADER))
}

fn compact_tables(table: &mut toml_edit::Table) {
    for (name, item) in table.iter_mut() {
        if matches!(name.get(), "sources" | "groups")
            && let Some(array) = item.as_array()
        {
            let mut tables = toml_edit::ArrayOfTables::new();
            for value in array.iter() {
                if let Some(value) = value.as_inline_table() {
                    tables.push(value.clone().into_table());
                }
            }
            *item = toml_edit::Item::ArrayOfTables(tables);
        }
        if name == "dependencies" {
            if let Some(table) = item.as_table_mut() {
                for (_, dependency) in table.iter_mut() {
                    dependency.make_value();
                }
            }
        } else if let Some(table) = item.as_table_mut() {
            compact_tables(table);
        } else if let Some(array) = item.as_array_of_tables_mut() {
            for table in array.iter_mut() {
                expand_inline_tables(table);
                compact_tables(table);
            }
        }
    }
}

fn expand_inline_tables(table: &mut toml_edit::Table) {
    for (name, item) in table.iter_mut() {
        if name == "dependencies"
            && let Some(value) = item.as_inline_table()
        {
            *item = toml_edit::Item::Table(value.clone().into_table());
        }
    }
}
