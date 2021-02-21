use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use binance::websockets::*;
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
}

#[derive(Debug, Clone)]
pub struct Ticker {
    name: String,
    average: f32,
    change: f32,
}

impl Ticker {
    pub fn title(&self) -> String {
        let size = match self.name.len() {
            6 => 3,
            7 => 4,
            8 => 4,
            _ => 3,
        };

        self.name[..size].to_string()
    }

    pub fn average_with_unit(&self) -> String {
        let size = match self.name.len() {
            6 => 3,
            7 => 4,
            8 => 4,
            _ => 3,
        };

        if self.name.contains("EUR") {
            format!("€{:.2}", self.average)
        } else if self.name.contains("USD") {
            format!("${:.2}", self.average)
        } else if self.name.contains("BRL") {
            format!("R${:.2}", self.average)
        } else {
            format!("{:.2} {}", self.average, &self.name[size..self.name.len()])
        }
    }
}

fn display(ticker: &Ticker, args: &Arguments) -> String {
    let formatted_change = format!("{:.1}%", ticker.change);

    format!(
        "{} {} ({}) ",
        colors::title(ticker.title(), args.polybar_mode),
        ticker.average_with_unit(),
        match ticker.change.partial_cmp(&0.0) {
            Some(Ordering::Greater) => colors::green(formatted_change, args.polybar_mode),
            Some(Ordering::Less) => colors::red(formatted_change, args.polybar_mode),
            Some(Ordering::Equal) => formatted_change,
            None => formatted_change,
        }
    )
}

fn main() -> Result<(), Error> {
    let args = Arguments::from_args();
    let keep_running = AtomicBool::new(true); // Used to control the event loop
    let agg_trade: String = format!("!ticker@arr"); // All Symbols

    let interested = vec!["BTCEUR", "BTCUSD", "BTCBRL", "ADAEUR", "ADABNB", "BNBEUR"];

    let mut averages: HashMap<String, Ticker> = HashMap::new();

    let mut web_socket: WebSockets = WebSockets::new(|event: WebsocketEvent| {
        match event {
            // 24hr rolling window ticker statistics for all symbols that changed in an array.
            WebsocketEvent::DayTickerAll(ticker_events) => {
                for tick_event in ticker_events {
                    if interested.iter().any(|&x| tick_event.symbol.clone() == x) {
                        let (average, change) = (
                            tick_event.average_price.parse::<f32>()?,
                            tick_event.price_change_percent.parse::<f32>()?,
                        );
                        let ticker = Ticker {
                            name: tick_event.symbol.clone(),
                            average,
                            change,
                        };
                        averages
                            .entry(tick_event.clone().symbol)
                            .and_modify(|x| *x = ticker.clone())
                            .or_insert(ticker);
                    }
                }

                for ticker in interested.iter() {
                    let ticker = match averages.get(&ticker.to_string()) {
                        Some(val) => val,
                        None => continue,
                    };

                    print!("{}", display(&ticker, &args));
                }

                println!("");
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
