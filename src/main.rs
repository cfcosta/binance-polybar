use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

use binance::websockets::*;
use colored::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Binance API Error")]
    ApiError(#[from] binance::errors::Error),
}

pub struct Pair<'t>(&'t str);

fn main() -> Result<(), Error> {
    let keep_running = AtomicBool::new(true); // Used to control the event loop
    let agg_trade: String = format!("!ticker@arr"); // All Symbols

    let interested = vec![
        Pair("ADAEUR"),
        Pair("BTCEUR"),
        Pair("DOTEUR"),
        Pair("CAKEBNB"),
        Pair("BNBEUR"),
    ];

    let mut averages: HashMap<String, f32> = HashMap::new();
    let mut current: HashMap<String, f32> = HashMap::new();
    let samples_to_average = 5.0;

    let mut web_socket: WebSockets = WebSockets::new(|event: WebsocketEvent| {
        match event {
            // 24hr rolling window ticker statistics for all symbols that changed in an array.
            WebsocketEvent::DayTickerAll(ticker_events) => {
                for tick_event in ticker_events {
                    if interested.iter().any(|x| tick_event.symbol == x.0) {
                        let average =
                            (1000.0 * tick_event.average_price.parse::<f32>()?).round() / 1000.0;
                        current
                            .entry(tick_event.symbol.clone())
                            .and_modify(|x| *x = average)
                            .or_insert(average);
                    }
                }

                for ticker in interested.iter() {
                    let average = match current.get(&ticker.0.to_string()) {
                        Some(val) => val,
                        None => continue,
                    };
                    let old_average = averages.get(&ticker.0.to_string()).unwrap_or(&average);

                    print!(
                        "{}: {} ",
                        ticker.0,
                        match average.partial_cmp(old_average) {
                            Some(cmp) => match cmp {
                                Ordering::Equal => format!("%{{F#e6e6e6}}{:.3}%{{F-}}", average),
                                Ordering::Greater => format!("%{{F#50fa7b}}{:.3}%{{F-}}", average),
                                Ordering::Less => format!("%{{F#ff5555}}{:.3}%{{F-}}", average),
                            },
                            None => format!("%{{F#e6e6e6}}{:.3}%{{F-}}", average),
                        }
                    );

                    averages
                        .entry(ticker.0.to_string())
                        .and_modify(|x| {
                            *x = *x + ((average - *x) / 2.0).round();
                        })
                        .or_insert(*average);
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
