use solana_address::Address;

pub fn abort() -> ! {
    unsafe {
        let syscall: extern "C" fn() -> ! = core::mem::transmute(3069975057usize);
        syscall();
    }
}

pub fn assign_owner(account_idx: u64, new_owner: *const Address) {
    unsafe {
        let syscall: extern "C" fn(u64, *const Address) = core::mem::transmute(4042720265u64);
        syscall(account_idx, new_owner);
    }
}

pub fn set_buffer_length(base_address: u64, new_length: u64) -> u64 {
    unsafe {
        let syscall: extern "C" fn(u64, u64, u64, u64, u64) -> u64 =
            core::mem::transmute(0x713026f5u64);
        syscall(base_address, new_length, 0, 0, 0)
    }
}

pub fn sol_log_(message: *const u8, len: u64) {
    unsafe {
        let syscall: extern "C" fn(*const u8, u64) = core::mem::transmute(544561597u64);
        syscall(message, len)
    }
}

pub fn sol_panic_(filename: *const u8, filename_len: u64, line: u64, column: u64) -> ! {
    unsafe {
        let syscall: extern "C" fn(*const u8, u64, u64, u64) -> ! =
            core::mem::transmute(1751159739usize);
        syscall(filename, filename_len, line, column);
    }
}

pub fn sol_transfer_lamports(to_account_tx_idx: u64, from_account_tx_idx: u64, lamports: u64) {
    unsafe {
        let syscall: extern "C" fn(u64, u64, u64) = core::mem::transmute(410538403u64);
        syscall(to_account_tx_idx, from_account_tx_idx, lamports);
    }
}
