use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use binance::websockets::*;
use colored::*;
use structopt::StructOpt;
use thiserror::Error;

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

fn main() -> Result<(), Error> {
    let args = Arguments::from_args();
    let keep_running = AtomicBool::new(true); // Used to control the event loop
    let agg_trade: String = format!("!ticker@arr"); // All Symbols

    let interested = vec!["BTCEUR", "BTCUSDT", "BTCBRL", "ADAEUR", "BNBEUR"];

    let mut averages: HashMap<String, (f32, f32)> = HashMap::new();

    let mut web_socket: WebSockets = WebSockets::new(|event: WebsocketEvent| {
        match event {
            // 24hr rolling window ticker statistics for all symbols that changed in an array.
            WebsocketEvent::DayTickerAll(ticker_events) => {
                for tick_event in ticker_events {
                    if interested.iter().any(|&x| tick_event.symbol == x) {
                        let (average, change) = (
                            tick_event.average_price.parse::<f32>()?,
                            tick_event.price_change_percent.parse::<f32>()?,
                        );
                        averages
                            .entry(tick_event.symbol.clone())
                            .and_modify(|x| *x = (average, change))
                            .or_insert((average, change));
                    }
                }

                for ticker in interested.iter() {
                    let (average, change) = match averages.get(&ticker.to_string()) {
                        Some(val) => val,
                        None => continue,
                    };

                    let formatted_change = format!("{:.1}%", change);

                    let green = if args.polybar_mode {
                        format!("%{{F#50fa7b}}{}%{{F-}}", formatted_change)
                    } else {
                        formatted_change.green().to_string()
                    };

                    let red = if args.polybar_mode {
                        format!("%{{F#ff5555}}{}%{{F-}}", formatted_change)
                    } else {
                        formatted_change.red().to_string()
                    };

                    let size = match ticker.len() {
                        6 => 3,
                        7 => 4,
                        8 => 4,
                        _ => 3,
                    };

                    let average_with_unit = if ticker.contains("EUR") {
                        format!("€{:.2}", average)
                    } else if ticker.contains("USD") {
                        format!("${:.2}", average)
                    } else if ticker.contains("BRL") {
                        format!("R${:.2}", average)
                    } else {
                        format!("{:.2} {}", average, &ticker[size..ticker.len()])
                    };

                    print!(
                        "{}: {} ({}) ",
                        &ticker[..size],
                        average_with_unit,
                        match change.partial_cmp(&0.0) {
                            Some(cmp) => match cmp {
                                Ordering::Equal => formatted_change,
                                Ordering::Greater => green,
                                Ordering::Less => red,
                            },
                            None => formatted_change,
                        }
                    );
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
