# Undead

A tool to search for dead code in your Python projects.

## Installation
### Using Cargo
```bash
$ cargo install undead
```

## Usage
```sh
$ undead . -I "tests"
```

## Documentation
```sh
$ undead --help
```

```plaintext
A tool to search for dead code in your Python projects

Usage: undead [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  paths in which to recursively search for dead files

Options:
  -I, --ignore-paths <IGNORE_PATHS>  paths to ignore when searching for dead files
  -h, --help                         Print help
  -V, --version                      Print version

```
