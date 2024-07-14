use crate::cli::Cli;
use clap::Parser;
use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{sinks::UTF8, Searcher},
};
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder};

use rayon::prelude::*;
use rustpython_ast::{Mod, ModModule, Stmt, StmtImport, StmtImportFrom};
use rustpython_parser::{parse, Mode};
use std::{
    collections::HashSet,
    path::{Path, PathBuf, MAIN_SEPARATOR_STR},
};

mod cli;

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let target_paths = cli.paths;
    let ignore_paths = cli.ignore_paths;

    let target_paths = parallel_build_path_iterator(&target_paths, &ignore_paths)?;
    let no_entrypoint_paths = target_paths
        .clone()
        .into_par_iter()
        .filter(|path| file_contains_name_equals_main(path).unwrap());

    let python_root = find_python_project_root(&target_paths[0]).unwrap();

    let all_paths = parallel_build_path_iterator(&vec![python_root], &ignore_paths)?;
    let imports = compile_imports(all_paths, &python_root)?;

    let imports_hash_set: HashSet<String> = imports.iter().cloned().collect();

    let potentially_dead_modules = no_entrypoint_paths
        .map(convert_path_to_module)
        .collect::<Vec<String>>();
    potentially_dead_modules
        .into_par_iter()
        .filter(|module| !imports_hash_set.contains(module))
        .for_each(|module| println!("{}", module));
    Ok(())
}

fn compile_imports(python_files: Vec<PathBuf>, python_root: &Path) -> anyhow::Result<Vec<Import>> {
    let imports_queue = SegQueue::<Import>::new();
    python_files
        .par_iter()
        .map(|path| match extract_imports(&path, &python_root) {
            Ok(imports) => {
                imports
                    .into_iter()
                    .for_each(|import| imports_queue.push(import));
                Ok(())
            }
            Err(_) => return Err(()),
        })
        .collect::<Vec<_>>();
    Ok(imports_queue.into_iter().collect())
}

enum Import {
    Module(String),
    Package(String),
}
impl Import {
    fn from_import(import: &StmtImport, python_root: &Path) -> Vec<Import> {
        import
            .names
            .iter()
            .map(|alias| {
                let alias_name = alias.name.to_string();
                let full_path = python_root.join(alias_name.replace(".", MAIN_SEPARATOR_STR));
                if full_path.is_dir() {
                    Import::Package(alias_name)
                } else {
                    Import::Module(alias_name)
                }
            })
            .collect()
    }

    fn from_import_from(
        import_from: &StmtImportFrom,
        current_file_path: &Path,
        python_root: &Path,
    ) -> Vec<Import> {
        let mut base_import_path: PathBuf;
        match import_from.level {
            Some(level) => {
                // absolute import
                if level.to_usize() == 0 {
                    base_import_path = python_root.to_path_buf();
                // relative import
                } else {
                    base_import_path = current_file_path.to_path_buf();
                    for _ in 0..level.to_usize() {
                        base_import_path = base_import_path.parent().unwrap().to_path_buf();
                    }
                }
            }
            // when does this happen?
            None => {
                base_import_path = python_root.to_path_buf();
            }
        }
        let mut full_import_path: PathBuf;
        if let Some(module) = import_from.module {
            full_import_path =
                base_import_path.join(module.to_string().replace(".", MAIN_SEPARATOR_STR));
            if full_import_path.is_file() {
                return vec![Import::Module(render_as_import_string(
                    &full_import_path,
                    python_root,
                ))];
            }
        }
        import_from
            .names
            .iter()
            .map(|alias| {
                let alias_name = alias.name.to_string();
                full_import_path = full_import_path.join(alias_name);
                let full_import = render_as_import_string(&full_import_path, python_root);
                if full_import_path.is_dir() {
                    Import::Package(full_import)
                } else {
                    Import::Module(full_import)
                }
            })
            .collect()
    }
}

fn render_as_import_string(path: &Path, python_root: &Path) -> String {
    path.to_string_lossy()
        .strip_prefix(python_root.to_string_lossy().as_ref())
        .unwrap()
        .to_string()
        .replace(MAIN_SEPARATOR_STR, ".")
}

// StmtImportFrom { range: 473..588, module: Some(Identifier("new_org.norms.international_tax_agreements.ingestion.common.s3")), names: [Alias { range: 554..585, name: Identifier("build_agreement_metadata_s3_key"), asname: None }], level: Some(Int(0)) }
fn extract_imports(path: &Path, python_root: &Path) -> anyhow::Result<Vec<Import>> {
    let file_contents = std::fs::read_to_string(path)?;
    match parse(&file_contents, Mode::Module, "<embedded>") {
        Ok(Mod::Module(ModModule {
            range: _,
            body,
            type_ignores: __,
        })) => {
            let imported_modules = body
                .iter()
                .map(|stmt| match stmt {
                    Stmt::Import(import) => Import::from_import(import, python_root),
                    Stmt::ImportFrom(import_from) => {
                        Import::from_import_from(import_from, path, python_root)
                    }
                    _ => vec![],
                })
                .flatten()
                .collect();
            Ok(imported_modules)
        }
        _ => Err(anyhow::anyhow!("Error parsing file: {:?}", path)),
    }
}

fn convert_path_to_module(path: PathBuf) -> String {
    path.to_string_lossy().replace(MAIN_SEPARATOR_STR, ".")
}

fn parallel_build_path_iterator(
    paths: &Vec<PathBuf>,
    ignore_paths: &Vec<PathBuf>,
) -> anyhow::Result<Vec<PathBuf>> {
    let walk_builder = walk_builder(paths, ignore_paths);
    let file_queue = SegQueue::<PathBuf>::new();
    walk_builder.build_parallel().run(|| {
        Box::new(
            |entry: Result<DirEntry, ignore::Error>| -> ignore::WalkState {
                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type().unwrap();
                        if !file_type.is_dir() {
                            file_queue.push(entry.path().to_path_buf());
                        }
                        ignore::WalkState::Continue
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        ignore::WalkState::Continue
                    }
                }
            },
        )
    });
    Ok(file_queue.into_iter().collect())
}

fn walk_builder(paths: &Vec<PathBuf>, ignore_paths: &Vec<PathBuf>) -> WalkBuilder {
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults().select("python");

    let mut walk_builder = WalkBuilder::new(&paths[0]);
    for path in paths.iter().skip(1) {
        walk_builder.add(path);
    }
    // FIXME: this doesn't work
    for ignore in ignore_paths.iter() {
        walk_builder.add_ignore(ignore);
    }
    walk_builder.types(types_builder.build().unwrap());
    walk_builder
}

fn file_contains_name_equals_main(path: &PathBuf) -> anyhow::Result<bool> {
    let matcher = RegexMatcher::new(r#"if\s+__name__\s*==\s*["']__main__["']:"#).unwrap();
    let mut matches = vec![];
    Searcher::new().search_path(
        &matcher,
        path,
        UTF8(|lnum, line| match matcher.find(line.as_bytes()) {
            Ok(Some(_)) => {
                matches.push((lnum, line.to_string()));
                return Ok(true);
            }
            Ok(None) => return Ok(false),
            Err(err) => return Err(err.into()),
        }),
    )?;
    if matches.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

fn is_python_project_root(dir: &Path) -> bool {
    let markers = vec!["setup.py", "pyproject.toml", ".git"];
    for marker in markers {
        if dir.join(marker).exists() {
            return true;
        }
    }
    false
}

/// Finds the root path of a Python project starting from a given directory.
fn find_python_project_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current_dir = start_dir;

    loop {
        if is_python_project_root(current_dir) {
            return Some(current_dir.to_path_buf());
        }

        // Move to the parent directory
        match current_dir.parent() {
            Some(parent) => current_dir = parent,
            None => break,
        }
    }

    None
}
