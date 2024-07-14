use std::fmt;
use std::io::Write;
use std::{io::IsTerminal, time::Duration};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use terminal_size::{terminal_size, Height, Width};

pub enum Printable<'a> {
    Message(String),
    Error(String),
    Stats(Stats<'a>),
    DeadFile(DeadFile<'a>),
    Separator,
}

const DEFAULT_SEPARATOR: &str = "-";
const DEFAULT_SEPARATOR_SIZE: u16 = 80;

pub trait Printer {
    fn print(&self, printable: Printable, stream: &mut StandardStream) -> std::io::Result<()> {
        if is_terminal() {
            self.print_generic(&printable, stream)
        } else {
            match printable {
                Printable::Message(msg) => println!("{}", msg),
                Printable::Error(err) => eprintln!("{}", err),
                Printable::Stats(stats) => println!("{:?}", stats),
                Printable::DeadFile(file) => println!("{}", file.repr),
                Printable::Separator => {
                    println!(
                        "\n{}\n",
                        DEFAULT_SEPARATOR.repeat(DEFAULT_SEPARATOR_SIZE as usize)
                    )
                }
            };
            Ok(())
        }
    }
    fn print_generic(
        &self,
        printable: &Printable,
        stream: &mut StandardStream,
    ) -> std::io::Result<()> {
        match printable {
            Printable::Message(msg) => self.print_message(msg, stream),
            Printable::Error(err) => self.print_error(err, stream),
            Printable::Stats(stats) => self.print_stats(stats, stream),
            Printable::DeadFile(file) => self.print_dead_file(file, stream),
            Printable::Separator => self.print_separator(stream),
        }
    }

    fn print_message(&self, msg: &str, stream: &mut StandardStream) -> std::io::Result<()>;
    fn print_error(&self, err: &str, stream: &mut StandardStream) -> std::io::Result<()>;
    fn print_stats(&self, stats: &Stats, stream: &mut StandardStream) -> std::io::Result<()>;
    fn print_dead_file(&self, file: &DeadFile, stream: &mut StandardStream) -> std::io::Result<()>;
    fn print_separator(&self, stream: &mut StandardStream) -> std::io::Result<()>;
}

fn is_terminal() -> bool {
    std::io::stdin().is_terminal()
}

pub struct TerminalPrinter;

impl Printer for TerminalPrinter {
    fn print_message(&self, msg: &str, stream: &mut StandardStream) -> std::io::Result<()> {
        stream.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        writeln!(stream, "{}", msg)
    }

    fn print_error(&self, err: &str, stream: &mut StandardStream) -> std::io::Result<()> {
        stream.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(stream, "{}", err)
    }

    fn print_stats(&self, stats: &Stats, stream: &mut StandardStream) -> std::io::Result<()> {
        stream.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(stream, "Found {} dead files", stats.dead_files)?;
        writeln!(
            stream,
            "Scanned {} files in {:?}",
            stats.scanned_files, stats.duration
        )
    }

    fn print_dead_file(&self, file: &DeadFile, stream: &mut StandardStream) -> std::io::Result<()> {
        stream.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        let link = Hyperlink {
            uri: &format!("file://{}", file.full_path),
            id: None,
        };
        writeln!(stream, "{link}{}{link:#}", file.repr)
    }

    fn print_separator(&self, stream: &mut StandardStream) -> std::io::Result<()> {
        let mut width = DEFAULT_SEPARATOR_SIZE;
        if let Some((Width(w), _)) = terminal_size() {
            width = w;
        }
        stream.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        writeln!(stream, "{}", DEFAULT_SEPARATOR.repeat(width as usize))
    }
}

#[derive(Debug)]
pub struct Stats<'a> {
    pub dead_files: &'a usize,
    pub scanned_files: &'a usize,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct DeadFile<'a> {
    pub repr: &'a str,
    pub full_path: &'a str,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Hyperlink<'a> {
    // maybe this should use u8 to support non-utf encodings?
    uri: &'a str,
    id: Option<&'a str>,
}

const OSC8: &str = "\x1b]8";

/// string terminator
const ST: &str = "\x1b\\";

impl fmt::Display for Hyperlink<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = self.uri;
        if f.alternate() {
            // based off of the cargo internal hyperlink behavior.
            // if the alternate flag is specified, end the hyperlink.
            write!(f, "{OSC8};;{ST}")
        } else if let Some(id) = self.id {
            write!(f, "{OSC8};id={id};{url}{ST}")
        } else {
            write!(f, "{OSC8};;{url}{ST}")
        }
    }
}
