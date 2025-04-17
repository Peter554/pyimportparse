use rayon::prelude::*;
use std::collections::HashMap;
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
    let _: HashMap<PathBuf, Vec<Import>> = modules_paths_to_scan
        .into_par_iter()
        .fold(
            HashMap::new,
            |mut hm: HashMap<PathBuf, Vec<Import>>, module_path| {
                let code = fs::read_to_string(&module_path).unwrap();
                let imports = parse_imports(&code).unwrap();
                hm.insert(module_path, imports);
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
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
