use libtock_emulation::syscall::Syscall;

#[inline(always)]
// Justification: documentation is generated from mocks
#[allow(clippy::missing_safety_doc)]
pub unsafe fn yieldk() {
    Syscall::new_yieldk().invoke();
}

#[inline(always)]
// Justification: documentation is generated from mocks
#[allow(clippy::missing_safety_doc)]
pub unsafe fn subscribe(
    major: usize,
    minor: usize,
    cb: *const unsafe extern "C" fn(usize, usize, usize, usize),
    ud: usize,
) -> isize {
    Syscall::new_subscribe(major, minor, cb, ud).invoke()
}

#[inline(always)]
// Justification: documentation is generated from mocks
#[allow(clippy::missing_safety_doc)]
pub unsafe fn command(major: usize, minor: usize, arg1: usize, arg2: usize) -> isize {
    Syscall::new_command(major, minor, arg1, arg2).invoke()
}

#[inline(always)]
// Justification: documentation is generated from mocks
#[allow(clippy::missing_safety_doc)]
pub unsafe fn command1(major: usize, minor: usize, arg: usize) -> isize {
    Syscall::new_command(major, minor, arg, 0).invoke()
}

#[inline(always)]
// Justification: documentation is generated from mocks
#[allow(clippy::missing_safety_doc)]
pub unsafe fn allow(major: usize, minor: usize, slice: *mut u8, len: usize) -> isize {
    Syscall::new_allow(major, minor, slice, len).invoke()
}

#[inline(always)]
// Justification: documentation is generated from mocks
#[allow(clippy::missing_safety_doc)]
pub unsafe fn memop(major: u32, arg1: usize) -> isize {
    Syscall::new_memop(major, arg1).invoke()
}
