use ansi_term::{ANSIString, ANSIStrings, Color, Style};
use clap::{Parser, ValueEnum};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum InputFile {
    Toml,
    Json,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Render the input file
    Render {
        /// Path to the input file
        #[arg(value_name = "FILE")]
        input: String,

        /// Input file type
        #[arg(value_enum, short = 't', long = "type")]
        input_type: Option<InputFile>,
    },

    /// Validate the input file
    Validate {
        /// Path to the input file
        #[arg(value_name = "FILE")]
        input: String,

        /// Input file type
        #[arg(value_enum, short = 't', long = "type")]
        input_type: Option<InputFile>,
    },

    /// Print the JSON schema for the input file
    Schema,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The command to run
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
enum InputStyle {
    Bold,
    Dimmed,
    Italic,
    Underlined,
    Blink,
    Reverse,
    Hidden,
    Strikethrough,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
enum InputColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
    Fixed(u8),
    RGB(u8, u8, u8),
}

impl InputColor {
    fn to_color(&self) -> Color {
        match self {
            InputColor::Black => Color::Black,
            InputColor::Red => Color::Red,
            InputColor::Green => Color::Green,
            InputColor::Yellow => Color::Yellow,
            InputColor::Blue => Color::Blue,
            InputColor::Purple => Color::Purple,
            InputColor::Cyan => Color::Cyan,
            InputColor::White => Color::White,
            InputColor::Fixed(n) => Color::Fixed(*n),
            InputColor::RGB(r, g, b) => Color::RGB(*r, *g, *b),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct InputStyles {
    foreground: Option<InputColor>,
    background: Option<InputColor>,
    styles: Option<Vec<InputStyle>>,
}

impl InputStyles {
    fn to_style(&self) -> Style {
        let mut style = Style::default();
        match &self.styles {
            Some(styles) => {
                for s in styles {
                    match s {
                        InputStyle::Bold => style = style.bold(),
                        InputStyle::Dimmed => style = style.dimmed(),
                        InputStyle::Italic => style = style.italic(),
                        InputStyle::Underlined => style = style.underline(),
                        InputStyle::Blink => style = style.blink(),
                        InputStyle::Reverse => style = style.reverse(),
                        InputStyle::Hidden => style = style.hidden(),
                        InputStyle::Strikethrough => style = style.strikethrough(),
                    }
                }
            }
            None => {}
        }

        match &self.foreground {
            Some(c) => style = style.fg(c.to_color()),
            None => {}
        }

        match &self.background {
            Some(c) => style = style.on(c.to_color()),
            None => {}
        }

        style
    }
}

type StyleMap = HashMap<char, InputStyles>;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct Input {
    pub design: String,
    pub styles: String,
    pub style_map: StyleMap,
}

fn render(design: &str, styles: &str, map: &StyleMap) -> String {
    design
        .lines()
        .zip(styles.lines())
        .into_iter()
        .map(|(block_line, style_line)| {
            assert_eq!(
                block_line.chars().count(),
                style_line.chars().count(),
                "Non-matching line lengths"
            );
            block_line
                .chars()
                .zip(style_line.chars())
                .map(|(b, s)| {
                    map.get(&s)
                        .unwrap_or_else(|| panic!("No style for char '{}'", s))
                        .to_style()
                        .paint(b.to_string())
                })
                .collect::<Vec<ANSIString>>()
        })
        .map(|line| ANSIStrings(&line).to_string())
        .collect::<Vec<String>>()
        .join("\n")
}

fn read_input(input: &str) -> Result<String, std::io::Error> {
    if input == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        fs::read_to_string(input)
    }
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Render { input, input_type } => {
            let input = read_input(&input).expect("Failed to read input file");
            let input: Input = match input_type {
                Some(InputFile::Json) => {
                    serde_json::from_str(input.as_str()).expect("Invalid JSON")
                }
                _ => toml::from_str(input.as_str()).expect("Invalid TOML"),
            };
            println!("{}", render(&input.design, &input.styles, &input.style_map));
        }
        Commands::Validate { input, input_type } => {
            let input_str = read_input(&input).expect("Failed to read input file");
            let input_json = match input_type {
                Some(InputFile::Json) => {
                    serde_json::from_str(input_str.as_str()).expect("Invalid JSON")
                }
                _ => {
                    let input_toml: toml::Value =
                        toml::from_str(input_str.as_str()).expect("Invalid TOML");
                    serde_json::to_value(&input_toml).unwrap()
                }
            };

            let schema = &schema_for!(Input);
            let schema_str = serde_json::to_string(schema).unwrap();
            let schema_json = serde_json::from_str(schema_str.as_str()).unwrap();

            jsonschema::validate(&schema_json, &input_json).unwrap_or_else(|e| {
                eprintln!("Validation failed: {}", e);
                std::process::exit(1);
            });

            eprintln!("Validation succeeded");
        }
        Commands::Schema => {
            let schema = schema_for!(Input);
            println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        }
    }
}
