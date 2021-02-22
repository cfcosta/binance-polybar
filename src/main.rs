use std::sync::atomic::{AtomicBool, Ordering};

use binance::websockets::*;
use indexmap::IndexMap;
use structopt::StructOpt;
use thiserror::Error;

mod colors;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Binance API Error")]
    ApiError(#[from] binance::errors::Error),
}

#[derive(Debug, StructOpt)]
pub struct Arguments {
    #[structopt(long = "polybar-mode", short = "p")]
    /// Outputs colors using polybar format strings
    polybar_mode: bool,

    #[structopt(long = "one-line", short = "o")]
    /// Prints one line and quits
    one_line: bool,
}

#[derive(Debug, Clone)]
pub struct Ticker {
    parent: ExchangeTicker,
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
            let formatted_change = format!("{:.1}%", ticker.change);

            output.push_str(&*format!(
                "{} ({}) ",
                ticker.average_with_unit(),
                match ticker.change.partial_cmp(&0.0) {
                    Some(std::cmp::Ordering::Greater) =>
                        colors::green(formatted_change, args.polybar_mode),
                    Some(std::cmp::Ordering::Less) =>
                        colors::red(formatted_change, args.polybar_mode),
                    Some(std::cmp::Ordering::Equal) => formatted_change,
                    None => formatted_change,
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

#[derive(Clone, Debug)]
pub struct ExchangeTicker {
    pub name: String,
    pub from: String,
    pub to: String,
}

impl ExchangeTicker {
    pub fn new<T: Into<String>>(name: T, from: T, to: T) -> Self {
        Self {
            name: name.into(),
            from: from.into(),
            to: to.into(),
        }
    }
}

fn main() -> Result<(), Error> {
    let args = Arguments::from_args();
    let keep_running = AtomicBool::new(true); // Used to control the event loop
    let agg_trade: String = format!("!ticker@arr"); // All Symbols

    let interested = vec![
        ExchangeTicker::new("BTCEUR", "BTC", "EUR"),
        ExchangeTicker::new("BTCUSDT", "BTC", "USD"),
        ExchangeTicker::new("BTCBRL", "BTC", "BRL"),
        ExchangeTicker::new("ADAEUR", "ADA", "EUR"),
        ExchangeTicker::new("ADAUSDT", "ADA", "USD"),
        ExchangeTicker::new("ADABTC", "ADA", "BTC"),
        ExchangeTicker::new("DOTEUR", "DOT", "EUR"),
        ExchangeTicker::new("DOTUSDT", "DOT", "USD"),
        ExchangeTicker::new("DOTBTC", "DOT", "BTC"),
    ];

    let mut web_socket: WebSockets = WebSockets::new(|event: WebsocketEvent| {
        match event {
            WebsocketEvent::DayTickerAll(ticker_events) => {
                let mut averages: IndexMap<String, Vec<Ticker>> = IndexMap::new();

                for tick_event in ticker_events {
                    if let Some(parent) = interested
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
                            .and_modify(|x| x.push(ticker.clone()))
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
