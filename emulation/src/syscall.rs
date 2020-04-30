use crate::config;
use std::mem::{transmute, size_of};
use std::os::unix::net::UnixDatagram;

enum SyscallNum {
    YIELD = 0,
    SUBSCIBE = 1,
    COMMAND = 2,
    ALLOW = 3,
    MEMOP = 4,
}

#[repr(packed)]
#[allow(unused)]
#[derive(Default)]
pub struct Syscall {
    identifier: usize,
    syscall_number: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
}

#[repr(packed)]
#[allow(unused)]
#[derive(Default, Debug, Copy, Clone)]
pub struct Callback {
    pc: usize,
    args: [usize; 4],
}

#[repr(packed)]
#[allow(unused)]
#[derive(Default, Debug, Copy, Clone)]
pub struct KernelReturn {
    identifier: usize,
    ret_val: isize,
    cb: Callback,
    allow_count: usize,
}

#[repr(packed)]
#[allow(unused)]
#[derive(Default, Debug, Copy, Clone)]
struct AllowedRegionPreamble {
    address: usize,
    length: usize,
    remaining_slices: isize,
}

fn get_identifier() -> usize {
    let lifetime = 0;
    match config::get_config(&lifetime) {
        Some(config) => config.identifier as usize,
        None => panic!("Configuration not set."),
    }
}

fn raw_invoke(sock: &UnixDatagram, raw: &[u8]) {
    match sock.send(raw) {
        Ok(len) => {
            if len != size_of::<Syscall>() {
                println!("Kernel didn't recive the full syscall message.");
            }
        }
        Err(e) => panic!("Syscall failed: {}", e),
    };
}

unsafe fn get_allow_slices(sock: &UnixDatagram) {
    loop {
        const PREAMBLE_SIZE: usize = size_of::<AllowedRegionPreamble>();
        let mut buf: [u8; PREAMBLE_SIZE] = [0; PREAMBLE_SIZE];
        let len = sock.recv(&mut buf).unwrap();
        if len != PREAMBLE_SIZE {
            println!("Weird preamble size {}", len);
        }
        let preamble: AllowedRegionPreamble = transmute(buf);
        println!("Get {:?}", preamble);
        if preamble.remaining_slices < 0 {
            break;
        }

        let slice: &mut [u8] = std::slice::from_raw_parts_mut(
            preamble.address as *mut u8,
            preamble.length
        );

        let len = sock.recv(slice).unwrap();
        if len != preamble.length {
            panic!("Slice length mismatch, expected {}, but got {}", preamble.length, len);
        }

        if preamble.remaining_slices <= 1 {
            break;
        }
    }
}

unsafe fn supply_allow_slices(sock_rx: &UnixDatagram, sock_tx: &UnixDatagram) {
    loop {
        const PREAMBLE_SIZE: usize = size_of::<AllowedRegionPreamble>();
        let mut buf: [u8; PREAMBLE_SIZE] = [0; PREAMBLE_SIZE];
        let len = sock_rx.recv(&mut buf).unwrap();
        if len != PREAMBLE_SIZE {
            println!("Weird preamble size {}", len);
        }
        let preamble: AllowedRegionPreamble = transmute(buf);
        println!("Send {:?}", preamble);
        if preamble.remaining_slices < 0 {
            break;
        }

        let slice: &mut [u8] = std::slice::from_raw_parts_mut(
            preamble.address as *mut u8,
            preamble.length
        );

        let len = sock_tx.send(slice).unwrap();
        if len != preamble.length {
            panic!("Kernel recieved slice mismatch, expected {}, but sent {}",
                   preamble.length, len);
        }

        if preamble.remaining_slices <= 1 {
            break;
        }
    }
}

impl Syscall {
    pub fn new_yieldk() -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::YIELD as usize,
            ..Default::default()
        }
    }

    pub fn new_subscribe(
        major: usize,
        minor: usize,
        cb: *const unsafe extern "C" fn(usize, usize, usize, usize),
        ud: usize,
    ) -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::SUBSCIBE as usize,
            arg0: major,
            arg1: minor,
            arg2: cb as usize,
            arg3: ud,
        }
    }

    pub fn new_command(major: usize, minor: usize, arg1: usize, arg2: usize) -> Syscall{
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::COMMAND as usize,
            arg0: major,
            arg1: minor,
            arg2: arg1,
            arg3: arg2,
            ..Default::default()
        }
    }

    pub fn new_allow(major: usize, minor: usize, slice: *mut u8, len: usize) -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::ALLOW as usize,
            arg0: major,
            arg1: minor,
            arg2: slice as usize,
            arg3: len,
            ..Default::default()
        }
    }

    pub fn new_memop(major: u32, arg1: usize) -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::MEMOP as usize,
            arg0: major as usize,
            arg1: arg1,
            ..Default::default()
        }
    }

    /// 1. Send syscall.
    /// 2. Respond to requests for allowed slices.
    /// 3. Wait for syscall return value.
    /// 4. Apply copied slices.
    pub fn invoke(self) -> isize {
        let as_bytes = unsafe {
            transmute::<Syscall, [u8; size_of::<Syscall>()]>(self)
        };

        let (kernel_rx, kernel_tx) = match config::get_config(&as_bytes) {
            Some(config) => (&config.kernel_socket_rx, &config.kernel_socket_tx),
            None => panic!("Configuration not set."),
        };
        raw_invoke(kernel_tx, &as_bytes);

        unsafe { supply_allow_slices(kernel_rx, kernel_tx); }

        const RETURN_SIZE: usize = size_of::<KernelReturn>();

        let mut buf: [u8; RETURN_SIZE]= [0; RETURN_SIZE];
        let len = match kernel_rx.recv(&mut buf) {
            Ok(len) => len,
            Err(e) => panic!("Syscall return failed {}", e),
        };
        if len != RETURN_SIZE {
            // TODO generate error
            println!("Weird syscall return size {}, should be {}", len, RETURN_SIZE);
        }

        unsafe { get_allow_slices(kernel_rx); }

        let kernel_return = unsafe {
            transmute::<[u8; RETURN_SIZE], KernelReturn>(buf)
        };
        println!("Kernel return {:?}", kernel_return);

        if kernel_return.cb.pc != 0 {
            // We are being issued a callback
            let fn_ptr = kernel_return.cb.pc as *const ();
            let callback_fn: extern "C" fn(usize, usize, usize, usize) = unsafe {
                transmute(fn_ptr)
            };
            let args = kernel_return.cb.args;
            callback_fn(args[0], args[1], args[2], args[3]);
            0
        } else {
            let val = kernel_return.ret_val;
            println!("Returned from kernel {}", val);
            kernel_return.ret_val
        }

    }
}

