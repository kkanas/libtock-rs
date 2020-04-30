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
        .get_matches();

    let id_str = arg_match.value_of("id").unwrap();
    let socket_rx_str = arg_match.value_of("socket_recv").unwrap();
    let socket_tx_str = arg_match.value_of("socket_send").unwrap();

    let id = id_str.parse::<usize>().unwrap();
    let socket_rx = Path::new(socket_rx_str);
    let socket_tx = Path::new(socket_tx_str);

    if !socket_tx.exists() {
        panic!("No such socket: {}", socket_tx_str);
    }

    config::set_config(id, socket_rx, socket_tx);
}
