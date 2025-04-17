use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env::args;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use walkdir::{DirEntry, WalkDir};

use pyimportparse::{Import, parse_imports};

fn main() {
    let path: PathBuf = args().nth(1).expect("Path missing").into();

    let modules_paths_to_scan = WalkDir::new(path)
        .into_iter()
        // Do not descent into hidden directories.
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_type().is_dir() {
                return None;
            }
            if !entry.file_name().to_str().unwrap().ends_with(".py") {
                return None;
            }
            Some(entry.path().to_owned())
        })
        .collect::<Vec<_>>();

    let start = Instant::now();
    let imports: HashMap<String, Vec<Import>> = modules_paths_to_scan
        .into_par_iter()
        .fold(
            HashMap::new,
            |mut hm: HashMap<String, Vec<Import>>, module_path| {
                let code = fs::read_to_string(&module_path).unwrap();
                let imports = parse_imports(&code).unwrap();
                hm.insert(module_path.to_str().unwrap().to_owned(), imports);
                hm
            },
        )
        .reduce(HashMap::new, |mut hm, h| {
            for (k, v) in h {
                hm.entry(k).or_default().extend(v);
            }
            hm
        });
    let duration = start.elapsed();
    println!("Time to scan imports: {:?}", duration);

    if let Some(outpath) = args().nth(2) {
        let imports = imports
            .into_iter()
            .map(|(module_path, imports)| {
                (
                    module_path,
                    imports
                        .into_iter()
                        .map(|import| SerializableImport {
                            imported_object: import.imported_object,
                            line_number: import.line_number,
                            typechecking_only: import.typechecking_only,
                        })
                        .collect::<HashSet<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();
        let json = serde_json::to_string(&SerializableImportsData { data: imports }).unwrap();
        if outpath == ":print" {
            println!("{}", json);
        } else {
            let outpath: PathBuf = outpath.into();
            fs::write(&outpath, &json).expect("Unable to write imports file");
            println!("Imports written to: {:?}", outpath);
        }
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SerializableImportsData {
    data: HashMap<String, HashSet<SerializableImport>>,
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
struct SerializableImport {
    imported_object: String,
    line_number: u32,
    typechecking_only: bool,
}
