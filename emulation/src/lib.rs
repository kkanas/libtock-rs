#[allow(unused_macros)]
#[cfg(not(feature = "emulation"))]
macro_rules! log {
    ( $( $x:expr ),* ) => {};
}

#[cfg(feature = "emulation")]
macro_rules! log {
    (  $lvl:expr, $( $x:expr ),* ) => {
        {
            use crate::config;
            let lvl = $lvl as u8;
            let log_lvl = match config::get_config() {
                Some(config) => config.log_level as u8,
                None => 0 as u8,
            };

            if ( lvl <= log_lvl ) {
                println!("APP  {:?}: {}", $lvl, format!($( $x),* ));
            }
        }
    }
}

#[allow(unused_macros)]
#[cfg(not(feature = "emulation"))]
macro_rules! log_error {
    ( $( $x:expr ),* ) => {};
}

#[allow(unused_macros)]
#[cfg(feature = "emulation")]
macro_rules! log_error {
    ( $( $x:expr ),* ) => {
        log!(config::LogLevel::ERROR, $( $x),* )
    }
}

#[allow(unused_macros)]
#[cfg(not(feature = "emulation"))]
macro_rules! log_warn {
    ( $( $x:expr ),* ) => {};
}

#[allow(unused_macros)]
#[cfg(feature = "emulation")]
macro_rules! log_warn {
    ( $( $x:expr ),* ) => {
        log!(config::LogLevel::WARNING, $( $x),* )
    }
}

#[allow(unused_macros)]
#[cfg(not(feature = "emulation"))]
macro_rules! log_info {
    ( $( $x:expr ),* ) => {};
}

#[allow(unused_macros)]
#[cfg(feature = "emulation")]
macro_rules! log_info {
    ( $( $x:expr ),* ) => {
        log!(config::LogLevel::INFO, $( $x),* )
    }
}

#[allow(unused_macros)]
#[cfg(not(feature = "emulation"))]
macro_rules! log_dbg {
    ( $( $x:expr ),* ) => {};
}

#[allow(unused_macros)]
#[cfg(feature = "emulation")]
macro_rules! log_dbg {
    ( $( $x:expr ),* ) => {
        log!(config::LogLevel::DEBUG, $( $x),* )
    }
}

pub mod syscall;

#[allow(unused)]
mod config;

use std::path::Path;

use clap::{App, Arg};

pub fn setup() {
    let arg_match = App::new("A libtock-rs process")
        .arg(
            Arg::with_name("id")
                .short("i")
                .long("id")
                .takes_value(true)
                .help("A unique integer identifier")
                .required(true),
        )
        .arg(
            Arg::with_name("socket_send")
                .short("tx")
                .long("socket_send")
                .takes_value(true)
                .help("Path to the kernel socket for sending")
                .required(true),
        )
        .arg(
            Arg::with_name("socket_recv")
                .short("rx")
                .long("socket_recv")
                .takes_value(true)
                .help("Path to the kernel socket for recieving")
                .required(true),
        )
        .arg(
            Arg::with_name("log")
                .short("l")
                .long("log")
                .takes_value(true)
                .help("Log level 0 for no logs, 1 errors, 2 warnings, 3 info, 4 dbg")
                .required(false),
        )
        .get_matches();

    let id_str = arg_match.value_of("id").unwrap();
    let socket_rx_str = arg_match.value_of("socket_recv").unwrap();
    let socket_tx_str = arg_match.value_of("socket_send").unwrap();
    let log_level = arg_match.value_of("log").unwrap_or("0");
    let log_level = log_level.parse::<u8>().unwrap();

    let id = id_str.parse::<usize>().unwrap();
    let socket_rx = Path::new(socket_rx_str);
    let socket_tx = Path::new(socket_tx_str);
    if !socket_tx.exists() {
        panic!("No such socket: {}", socket_tx_str);
    }

    config::set_config(id, socket_rx, socket_tx, config::LogLevel::from(log_level));
}
