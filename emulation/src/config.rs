use std::cell::RefCell;
use std::collections::HashSet;
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::string::String;

#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    NONE,
    ERROR,
    WARNING,
    INFO,
    DEBUG,
}

impl LogLevel {
    pub fn from(lvl: u8) -> LogLevel {
        if lvl > 4 {
            return LogLevel::DEBUG;
        }
        match lvl {
            0 => LogLevel::NONE,
            1 => LogLevel::ERROR,
            2 => LogLevel::WARNING,
            3 => LogLevel::INFO,
            4 => LogLevel::DEBUG,
            _ => LogLevel::NONE,
        }
    }
}

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

    pub log_level: LogLevel,

    /// Set of addresses ALLOW'ED to Kernel
    pub allow_set: RefCell<HashSet<*const u8>>,
}

static mut APP_CONFIG: Option<Box<AppConfig>> = None;

pub fn set_config(identifier: usize, socket_rx: &Path, socket_tx: &Path, log_level: LogLevel) {
    unsafe {
        if APP_CONFIG.is_some() {
            panic!("Cannot configure more than once.");
        }
    }

    let rx = match UnixDatagram::bind(socket_rx) {
        Ok(sock) => sock,
        Err(e) => panic!("Couldn't open socket: {}", e),
    };

    let tx = UnixDatagram::unbound().unwrap();
    match tx.connect(socket_tx) {
        Ok(sock) => sock,
        Err(e) => panic!("Couldn't open socket: {}", e),
    };

    let config = Box::new(AppConfig {
        identifier,
        kernel_socket_rx: rx,
        kernel_socket_tx: tx,
        log_level,
        allow_set: RefCell::new(HashSet::new()),
    });

    // Log macros are working only after APP_CONFIG is set
    unsafe {
        APP_CONFIG = Some(config);
    }
    log_info!("Setup done");
}

pub fn get_config() -> Option<&'static AppConfig> {
    unsafe {
        match &APP_CONFIG {
            Some(config) => Some(&config),
            None => None,
        }
    }
}

pub fn get_config_or_panic() -> &'static AppConfig {
    unsafe {
        match &APP_CONFIG {
            Some(config) => &config,
            None => panic!("APP : Configuration not set."),
        }
    }
}
