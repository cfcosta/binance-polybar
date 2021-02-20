use std::fmt::Display;

use colored::*;

#[derive(PartialEq, Eq)]
pub enum Color {
    Normal,
    Red,
    Green,
    Purple,
    White,
    Black,
    Blue,
}

pub struct ColorPair {
    pub bg: Color,
    pub fg: Color,
}

impl ColorPair {
    pub fn new(bg: Color, fg: Color) -> Self {
        Self { bg, fg }
    }
}

pub fn green<T: Into<String>>(input: T, polybar_mode: bool) -> String {
    colorize(
        input,
        ColorPair::new(Color::Normal, Color::Green),
        polybar_mode,
    )
}

pub fn red<T: Into<String>>(input: T, polybar_mode: bool) -> String {
    colorize(
        input,
        ColorPair::new(Color::Normal, Color::Red),
        polybar_mode,
    )
}

pub fn title<T: Into<String> + Display>(input: T, polybar_mode: bool) -> String {
    colorize(
        format!(" {} ", input),
        ColorPair::new(Color::Blue, Color::Black),
        polybar_mode,
    )
}

fn to_palette<'a>(color: Color) -> &'a str {
    match color {
        Color::Normal => "#d8dee9",
        Color::Green => "#50fa7b",
        Color::Red => "#ff5555",
        Color::Purple => "#bd93f9",
        Color::White => "#fff",
        Color::Black => "#2e3440",
        Color::Blue => "#80a1c1",
    }
}

fn colorize<T: Into<String>>(input: T, color: ColorPair, polybar_mode: bool) -> String {
    if polybar_mode {
        let mut output = input.into().clone();

        if color.fg != Color::Normal {
            output = format!("%{{F{}}}{}%{{F-}}", to_palette(color.fg), output);
        }

        if color.bg != Color::Normal {
            output = format!("%{{B{}}}{}%{{B-}}", to_palette(color.bg), output);
        }

        output
    } else {
        let inp = input.into();

        let with_fg = match color.fg {
            Color::Red => inp.red(),
            Color::Green => inp.green(),
            Color::Purple => inp.blue(),
            Color::Normal => inp.normal(),
            Color::White => inp.white(),
            Color::Black => inp.black(),
            Color::Blue => inp.blue(),
        }
        .to_string();

        match color.bg {
            Color::Red => with_fg.on_red(),
            Color::Green => with_fg.on_green(),
            Color::Purple => with_fg.on_blue(),
            Color::Normal => with_fg.bold(),
            Color::White => with_fg.on_white(),
            Color::Black => inp.on_black(),
            Color::Blue => inp.on_blue(),
        }
        .to_string()
    }
}
