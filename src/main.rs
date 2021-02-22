use std::sync::atomic::{AtomicBool, Ordering};

use binance::websockets::*;
use indexmap::IndexMap;
use structopt::StructOpt;
use thiserror::Error;

mod colors;
mod config;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Binance API Error")]
    ApiError(#[from] binance::errors::Error),

    #[error("I/O Error")]
    IOError(#[from] std::io::Error),

    #[error("Failed to expand path: {0}")]
    PathExpansionError(String),

    #[error("Failed to parse configuration file")]
    ParseError,
}

#[derive(Debug, StructOpt)]
pub struct Arguments {
    #[structopt(long = "polybar-mode", short = "p")]
    /// Outputs colors using polybar format strings
    polybar_mode: bool,

    #[structopt(long = "one-line", short = "o")]
    /// Prints one line and quits
    one_line: bool,

    #[structopt(
        long = "config-file",
        short = "c",
        default_value = "~/.config/binance-i3status/config.toml"
    )]
    /// Where to find the configuration file
    config_file: String,
}

#[derive(Debug, Clone)]
pub struct Ticker {
    parent: config::Ticker,
    average: f32,
    change: f32,
}

impl Ticker {
    pub fn average_with_unit(&self) -> String {
        let avg = if self.average < 0.0001 {
            format!("{:.6}", self.average)
        } else if self.average < 0.01 {
            format!("{:.4}", self.average)
        } else {
            format!("{:.2}", self.average)
        };

        let to = &self.parent.to;

        if to.contains("EUR") {
            format!("€{}", avg)
        } else if to.contains("USD") {
            format!("${}", avg)
        } else if to.contains("BRL") {
            format!("R${}", avg)
        } else {
            format!("{} {}", avg, to)
        }
    }
}

fn display(tickers: &Vec<Ticker>, args: &Arguments) -> String {
    if tickers.len() > 1 {
        let mut output = colors::title(
            tickers.get(0).unwrap().clone().parent.from,
            args.polybar_mode,
        );
        output.push(' ');

        for ticker in tickers {
            use std::cmp::Ordering::*;

            let formatted_change = format!("{:.1}%", ticker.change);

            output.push_str(&*format!(
                "{} ({}) ",
                ticker.average_with_unit(),
                match ticker.change.partial_cmp(&0.0).unwrap_or(Equal) {
                    Greater => colors::green(formatted_change, args.polybar_mode),
                    Less => colors::red(formatted_change, args.polybar_mode),
                    Equal => formatted_change,
                }
            ));
        }

        output
    } else {
        let ticker = tickers.get(0).unwrap();
        let formatted_change = format!("{:.1}%", ticker.change);

        format!(
            "{} {} ({}) ",
            colors::title(ticker.parent.clone().from, args.polybar_mode),
            ticker.average_with_unit(),
            match ticker.change.partial_cmp(&0.0) {
                Some(std::cmp::Ordering::Greater) =>
                    colors::green(formatted_change, args.polybar_mode),
                Some(std::cmp::Ordering::Less) => colors::red(formatted_change, args.polybar_mode),
                Some(std::cmp::Ordering::Equal) => formatted_change,
                None => formatted_change,
            }
        )
    }
}

fn main() -> Result<(), Error> {
    let args = Arguments::from_args();
    let keep_running = AtomicBool::new(true); // Used to control the event loop
    let agg_trade: String = format!("!ticker@arr"); // All Symbols

    config::create_if_not_exists(&args.config_file)?;
    let config = config::parse(&args.config_file)?;

    let mut averages: IndexMap<String, Vec<Ticker>> = IndexMap::new();

    let mut web_socket: WebSockets = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::DayTickerAll(ticker_events) => {
                for tick_event in ticker_events {
                    let symbol = tick_event.symbol.clone();

                    if let Some(parent) = config
                        .tickers
                        .iter()
                        .find(|x| x.name == tick_event.symbol.clone())
                    {
                        let (average, change) = (
                            tick_event.average_price.parse::<f32>()?,
                            tick_event.price_change_percent.parse::<f32>()?,
                        );

                        let ticker = Ticker {
                            parent: parent.clone(),
                            average,
                            change,
                        };

                        averages
                            .entry(ticker.parent.from.clone())
                            .and_modify(|x| {
                                if let Some(idx) = x.iter().position(|x| x.parent.name == symbol) {
                                    x[idx] = ticker.clone();
                                } else {
                                    x.push(ticker.clone());
                                }
                            })
                            .or_insert(vec![ticker]);
                    }
                }

                for (_, tickers) in &averages {
                    print!("{}", display(&tickers, &args));
                }

                println!("");

                if args.one_line {
                    keep_running.store(false, Ordering::Relaxed);
                }
            }
            _ => (),
        };

        Ok(())
    });

    web_socket.connect(&agg_trade).unwrap(); // check error
    if let Err(e) = web_socket.event_loop(&keep_running) {
        match e {
            err => {
                println!("Error: {}", err);
            }
        }
    }

    Ok(())
}
