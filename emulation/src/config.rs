use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::string::String;

pub struct AppConfig {
    /// This unique identifier for this process. This is analogous to the Tock
    /// kernel's `identifier` part of `AppId`. Emulated processes send this as
    /// a part of syscalls and gives the kernel a way to identify apps across
    /// the syscall boundary since the stack pointer doesn't make sense in a
    /// virtual address space.
    pub identifier: usize,

    /// The Unix socket for recieving with the kernel.
    pub kernel_socket_rx: UnixDatagram,

    /// The Unix socket for sending to the kernel.
    pub kernel_socket_tx: UnixDatagram,
}

static mut APP_CONFIG: Option<Box<AppConfig>> = None;

pub fn set_config(identifier: usize, socket_rx: &Path, socket_tx: &Path) {
    unsafe {
        if APP_CONFIG.is_some() {
            panic!("Cannot configure more than once.");
        }
    }

    println!("Connecting rx to {:?}", socket_rx);
    let rx = match UnixDatagram::bind(socket_rx) {
        Ok(sock) => sock,
        Err(e) => panic!("Couldn't open socket: {}", e),
    };

    let tx = UnixDatagram::unbound().unwrap();
    println!("Connecting tx to {:?}", socket_tx);
    match tx.connect(socket_tx) {
        Ok(sock) => sock,
        Err(e) => panic!("Couldn't open socket: {}", e),
    };

    println!("{:?}", rx.local_addr());
    println!("{:?}", tx.peer_addr());

    let config = Box::new(AppConfig {
        identifier,
        kernel_socket_rx: rx,
        kernel_socket_tx: tx,
    });

    unsafe {
        APP_CONFIG = Some(config);
    }
}

pub fn get_config<'a, T>(_lifetime: &'a T) -> Option<&'a AppConfig> {
    unsafe {
        match &APP_CONFIG {
            Some(config) => Some(&config),
            None => None,
        }
    }
}
