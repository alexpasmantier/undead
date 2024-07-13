use crate::cli::Cli;
use clap::Parser;
use crossbeam::queue::SegQueue;
use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{sinks::UTF8, Searcher},
};
use ignore::{types::TypesBuilder, DirEntry, WalkBuilder};
use std::path::PathBuf;

mod cli;

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let paths = cli.paths;

    let potentially_dead_files = find_candidate_files(&paths)?;
    potentially_dead_files.into_iter().for_each(deal_with_entry);
    Ok(())
}

fn find_candidate_files(paths: &Vec<PathBuf>) -> anyhow::Result<SegQueue<PathBuf>> {
    let walk_builder = walk_builder(paths);
    let potentially_dead_files = SegQueue::<PathBuf>::new();
    walk_builder.build_parallel().run(|| {
        Box::new(
            |entry: Result<DirEntry, ignore::Error>| -> ignore::WalkState {
                match entry {
                    Ok(entry) => {
                        let file_type = entry.file_type().unwrap();
                        if !file_type.is_dir()
                            && !file_contains_name_equals_main(&entry).unwrap_or(false)
                        {
                            potentially_dead_files.push(entry.path().to_path_buf());
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
    Ok(potentially_dead_files)
}

fn deal_with_entry(path: PathBuf) -> () {
    println!("{:?}", path);
}

fn walk_builder(paths: &Vec<PathBuf>) -> WalkBuilder {
    let mut types_builder = TypesBuilder::new();
    types_builder.add_defaults().select("python");

    let mut walk_builder = WalkBuilder::new(&paths[0]);
    for path in paths.iter().skip(1) {
        walk_builder.add(path);
    }
    walk_builder.types(types_builder.build().unwrap());
    walk_builder
}

fn file_contains_name_equals_main(entry: &ignore::DirEntry) -> anyhow::Result<bool> {
    let matcher = RegexMatcher::new(r#"if\s+__name__\s*==\s*["']__main__["']:"#).unwrap();
    let mut matches = vec![];
    Searcher::new().search_path(
        &matcher,
        entry.path(),
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
