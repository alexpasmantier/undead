# Undead

A tool to search for dead code in your Python projects.

## Installation
### Using Cargo
```bash
$ cargo install undead
```

## Usage
```
$ undead . -I "tests"
```

## Documentation
```bash
$ undead --help
A tool to search for dead code in your Python projects

Usage: undead [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  paths in which to recursively search for dead files

Options:
  -I, --ignore-globs <IGNORE_GLOBS>  globs to ignore when searching for dead files
  -h, --help                         Print help
  -V, --version                      Print version

```
