use crate::config;
use std::mem::transmute;
use std::os::unix::net::UnixDatagram;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned};

enum SyscallNum {
    YIELD,
    SUBSCIBE,
    COMMAND,
    ALLOW,
    MEMOP,
}

#[repr(C, packed)]
#[derive(Unaligned, AsBytes, FromBytes, Default, Debug, Copy, Clone)]
pub struct Syscall {
    identifier: usize,
    pub syscall_number: usize,
    pub args: [usize; 4],
}

#[repr(C, packed)]
#[derive(Unaligned, AsBytes, FromBytes, Default, Debug, Copy, Clone)]
pub struct Callback {
    pc: usize,
    args: [usize; 4],
}

#[repr(C, packed)]
#[derive(Unaligned, AsBytes, FromBytes, Default, Debug, Copy, Clone)]
pub struct KernelReturn {
    ret_val: isize,
    cb: Callback,
}

#[repr(C, packed)]
#[derive(Unaligned, AsBytes, FromBytes, Default, Debug, Copy, Clone)]
pub struct AllowsInfo {
    pub number_of_slices: usize,
}

#[repr(C, packed)]
#[derive(Unaligned, AsBytes, FromBytes, Default, Debug, Copy, Clone)]
pub struct AllowSliceInfo {
    address: usize,
    length: usize,
}

pub const IPC_MSG_HDR_MAGIC: u16 = 0xA55A;

#[repr(C)]
pub enum IpcMsgType {
    SYSCALL,
    KERNELRETURN,
    ALLOWSINFO,
    ALLOWSLICEINFO,
}

pub trait IntoIpcMsgType {
    fn to_ipc_msg_type() -> IpcMsgType;
}

#[repr(C, packed)]
#[derive(Unaligned, AsBytes, FromBytes, Default, Debug, Copy, Clone)]
pub struct IpcMsgHeader {
    pub magic: u16,
    pub msg_len: u16,
    pub msg_type: u16,
    pub msg_cksum: u16,
}

impl IpcMsgHeader {
    pub fn new(msg_len: u16, msg_type: u16) -> IpcMsgHeader {
        IpcMsgHeader {
            magic: IPC_MSG_HDR_MAGIC,
            msg_len,
            msg_type,
            msg_cksum: IPC_MSG_HDR_MAGIC + msg_len + msg_type,
        }
    }
}

impl IntoIpcMsgType for Syscall {
    fn to_ipc_msg_type() -> IpcMsgType {
        IpcMsgType::SYSCALL
    }
}

impl IntoIpcMsgType for KernelReturn {
    fn to_ipc_msg_type() -> IpcMsgType {
        IpcMsgType::KERNELRETURN
    }
}

impl IntoIpcMsgType for AllowsInfo {
    fn to_ipc_msg_type() -> IpcMsgType {
        IpcMsgType::ALLOWSINFO
    }
}

impl IntoIpcMsgType for AllowSliceInfo {
    fn to_ipc_msg_type() -> IpcMsgType {
        IpcMsgType::ALLOWSLICEINFO
    }
}

fn get_identifier() -> usize {
    match config::get_config() {
        Some(config) => config.identifier as usize,
        None => panic!("Configuration not set."),
    }
}

fn send_allow_slice(socket: &UnixDatagram, allow: &Syscall) {
    if allow.syscall_number != SyscallNum::ALLOW as usize {
        return;
    }

    let address = allow.args[2] as *const u8;
    let length = allow.args[3];

    let slices_info = AllowsInfo {
        number_of_slices: 1,
    };
    let slice_info = AllowSliceInfo {
        address: address as usize,
        length,
    };

    send_msg(socket, get_identifier(), &slices_info);
    send_msg(socket, get_identifier(), &slice_info);
    unsafe {
        let slice: &mut [u8] = std::slice::from_raw_parts_mut(address as *mut u8, length);
        send_bytes(socket, slice);
    }
}

fn recv_allow_slices(sock: &UnixDatagram) {
    let slices_count: AllowsInfo = recv_msg(&sock);
    let slices_count = slices_count.number_of_slices;

    for _ in 0..slices_count {
        let allow_slice: AllowSliceInfo = recv_msg(&sock);
        unsafe {
            let slice: &mut [u8] =
                std::slice::from_raw_parts_mut(allow_slice.address as *mut u8, allow_slice.length);
            let rx_len = recv_bytes(sock, slice);
            if rx_len != allow_slice.length {
                panic!(
                    "APP : Slice length mismatch, expected {}, but got {}",
                    allow_slice.length, rx_len
                );
            }
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
            args: [major, minor, cb as usize, ud],
        }
    }

    pub fn new_command(major: usize, minor: usize, arg1: usize, arg2: usize) -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::COMMAND as usize,
            args: [major, minor, arg1, arg2],
        }
    }

    pub fn new_allow(major: usize, minor: usize, slice: *mut u8, len: usize) -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::ALLOW as usize,
            args: [major, minor, slice as usize, len],
        }
    }

    pub fn new_memop(major: u32, arg1: usize) -> Syscall {
        Syscall {
            identifier: get_identifier(),
            syscall_number: SyscallNum::MEMOP as usize,
            args: [major as usize, arg1, 0, 0],
        }
    }

    /// 1. Send syscall.
    /// 2. Send allow slice if syscall == ALLOW
    /// 3. Wait for syscall return value.
    pub fn invoke(self) -> isize {
        log_dbg!("SYSCALL: {:?}", self);
        let (kernel_rx, kernel_tx) = match config::get_config() {
            Some(config) => (&config.kernel_socket_rx, &config.kernel_socket_tx),
            None => panic!("APP : Configuration not set."),
        };

        send_msg(kernel_tx, get_identifier(), &self);
        send_allow_slice(kernel_tx, &self);
        let kernel_return: KernelReturn = recv_msg(&kernel_rx);

        recv_allow_slices(kernel_rx);

        if kernel_return.cb.pc != 0 {
            // We are being issued a callback
            let fn_ptr = kernel_return.cb.pc as *const ();
            let callback_fn: extern "C" fn(usize, usize, usize, usize) =
                unsafe { transmute(fn_ptr) };
            let args = kernel_return.cb.args;
            callback_fn(args[0], args[1], args[2], args[3]);
            0
        } else {
            kernel_return.ret_val
        }
    }
}

pub fn send_raw(socket: &UnixDatagram, bytes: &[u8]) {
    let sent = match socket.send(bytes) {
        Ok(len) => len,
        Err(e) => {
            panic!("socket send err {}", e);
        }
    };
    if sent != bytes.len() {
        panic!(
            "EmulationError send partialMessage {} expected {} ",
            sent,
            bytes.len()
        );
    }
}

pub fn send_bytes(socket: &UnixDatagram, bytes: &[u8]) {
    log_dbg!("SEND: bytes {:X?}", bytes);
    send_raw(socket, bytes);
}

pub fn send_msg<T>(socket: &UnixDatagram, _id: usize, msg: &T)
where
    T: AsBytes + Sized + IntoIpcMsgType + std::fmt::Debug,
{
    let ipc_len = std::mem::size_of::<T>() as u16;
    let ipc_type = T::to_ipc_msg_type() as u16;
    let ipc_hdr = IpcMsgHeader::new(ipc_len, ipc_type);

    log_dbg!("HDR: SEND: msg {:x?}", ipc_hdr);
    send_raw(socket, ipc_hdr.as_bytes());
    log_dbg!("SEND: msg {:x?}", msg);
    send_raw(socket, msg.as_bytes());
}

pub fn recv_bytes(sock: &UnixDatagram, buf: &mut [u8]) -> usize {
    let rx_len = match sock.recv(buf) {
        Ok(len) => len,
        Err(e) => {
            panic!("APP  : recv_bytes: error {}", e);
        }
    };
    if rx_len != buf.len() {
        panic!(
            "APP  : recv_bytes got {} bytes expected {}",
            rx_len,
            buf.len()
        );
    }
    return rx_len;
}

pub fn recv_raw(sock: &UnixDatagram, buf: &mut std::vec::Vec<u8>) {
    let len = sock.recv(buf.as_mut_slice()).unwrap();
    if len != buf.len() {
        panic!("Received bytes {} expected {}", len, buf.len());
    }
}

pub fn recv_msg<T>(sock: &UnixDatagram) -> T
where
    T: Sized + Clone + FromBytes + Unaligned + IntoIpcMsgType + std::fmt::Debug,
{
    let mut buf: Vec<u8> = Vec::new();
    let msg_len = std::mem::size_of::<IpcMsgHeader>();
    buf.resize_with(msg_len, Default::default);

    recv_raw(sock, &mut buf);

    let ipc_hdr: &IpcMsgHeader =
        match LayoutVerified::<_, IpcMsgHeader>::new_unaligned(buf.as_mut_slice()) {
            Some(hdr) => hdr.into_ref(),
            None => {
                panic!("Wrong bytes {:x?}", buf.as_slice());
            }
        };

    if ipc_hdr.magic != IPC_MSG_HDR_MAGIC {
        panic!("APP  : Wrong hdr magic {:x?}", ipc_hdr);
    }
    if ipc_hdr.msg_len != std::mem::size_of::<T>() as u16 {
        panic!(
            "APP  : Wrong hdr {:x?} len expected {} ",
            ipc_hdr,
            std::mem::size_of::<T>() as u16
        );
    }
    if ipc_hdr.msg_type != T::to_ipc_msg_type() as u16 {
        panic!(
            "APP  :Wrong hdr {:x?} type expected {} ",
            ipc_hdr,
            T::to_ipc_msg_type() as u16
        );
    }
    if ipc_hdr.msg_cksum != ipc_hdr.magic + ipc_hdr.msg_len + ipc_hdr.msg_type {
        panic!(
            "APP  : Wrong hdr {:x?} chksum expected {} ",
            ipc_hdr,
            ipc_hdr.magic + ipc_hdr.msg_len + ipc_hdr.msg_type
        );
    }

    let msg_len = std::mem::size_of::<T>();
    buf.resize_with(msg_len, Default::default);
    recv_raw(sock, &mut buf);

    let msg: &T = match LayoutVerified::<_, T>::new_unaligned(buf.as_mut_slice()) {
        Some(msg) => msg.into_ref(),
        None => {
            panic!("Wrong bytes {:x?}", buf.as_slice());
        }
    };
    log_dbg!("RECV: {:x?}", msg);
    return msg.clone();
}
