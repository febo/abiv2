use solana_address::Address;
pub use solana_define_syscall::definitions::{
    abort, sol_alt_bn128_compression, sol_alt_bn128_group_op, sol_big_mod_exp, sol_blake3,
    sol_create_program_address, sol_curve_decompress, sol_curve_group_op,
    sol_curve_multiscalar_mul, sol_curve_pairing_map, sol_curve_validate_point,
    sol_get_fees_sysvar, sol_get_stack_height, sol_get_sysvar, sol_keccak256, sol_log_,
    sol_log_64_, sol_log_compute_units_, sol_log_data, sol_log_pubkey, sol_memcmp_, sol_memcpy_,
    sol_memmove_, sol_memset_, sol_panic_, sol_poseidon, sol_remaining_compute_units,
    sol_secp256k1_recover, sol_sha256, sol_sha512, sol_try_find_program_address,
};

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

pub fn sol_invoke_signed(program_idx: u64, signer_seeds_ptr: u64, signer_seeds_len: u64) {
    unsafe {
        let syscall: extern "C" fn(u64, u64, u64) = core::mem::transmute(2722332484u64);
        syscall(program_idx, signer_seeds_ptr, signer_seeds_len);
    }
}

pub fn sol_transfer_lamports(to_account_tx_idx: u64, from_account_tx_idx: u64, lamports: u64) {
    unsafe {
        let syscall: extern "C" fn(u64, u64, u64) = core::mem::transmute(410538403u64);
        syscall(to_account_tx_idx, from_account_tx_idx, lamports);
    }
}
