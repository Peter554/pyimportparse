use itertools::Itertools;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use walkdir::{DirEntry, WalkDir};

// use pyimports::parse::parse_imports;
use pyimportparse::parse_imports;

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ImportsData {
    data: HashMap<String, HashSet<Import>>,
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
struct Import {
    imported: String,
    typechecking_only: bool,
}

fn main() {
    let mut data = HashMap::new();

    for entry in WalkDir::new("vendor/django/django")
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_name().to_str().unwrap().ends_with(".py") {
            continue;
        }

        let code = fs::read_to_string(entry.path()).unwrap();
        let imports = parse_imports(&code).unwrap();

        data.insert(
            entry.path().to_str().unwrap().to_owned(),
            imports
                .into_iter()
                .map(|i| Import {
                    imported: i.imported_object,
                    typechecking_only: i.typechecking_only,
                })
                .collect(),
        );
    }

    let imports_data = ImportsData { data };

    let expected_imports_data = fs::read_to_string("vendor/django/imports.json").unwrap();
    let expected_imports_data: ImportsData = serde_json::from_str(&expected_imports_data).unwrap();

    assert_eq!(
        expected_imports_data.data.keys().collect::<HashSet<_>>(),
        imports_data.data.keys().collect::<HashSet<_>>()
    );
    for key in expected_imports_data.data.keys() {
        println!("{}", key);
        assert_eq!(
            expected_imports_data
                .data
                .get(key)
                .unwrap()
                .iter()
                .sorted()
                .collect::<Vec<_>>(),
            imports_data
                .data
                .get(key)
                .unwrap()
                .iter()
                .sorted()
                .collect::<Vec<_>>()
        );
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
