use {
    mollusk_svm::{
        result::{Check, InstructionResult},
        Mollusk,
    },
    solana_account::Account,
    solana_address::Address,
    solana_instruction::Instruction,
};

/// Base lamports for accounts, used to ensure accounts are rent-exempt.
pub const BASE_LAMPORTS: u64 = 2_000_000_000u64;

pub const PROGRAM_ID: Address = Address::new_from_array([255u8; 32]);

pub fn run(
    mollusk: &Mollusk,
    instruction: &Instruction,
    accounts: &[(Address, Account)],
    checks: &[Check],
) -> (Address, InstructionResult) {
    let [(account_key, _), ..] = accounts else {
        panic!("expected at least one account");
    };

    let result = mollusk.process_and_validate_instruction(instruction, accounts, checks);

    let account = result.get_account(account_key);
    assert!(account.is_some());

    (*account_key, result)
}

/// Create a new Mollusk instance for the given program ID and name.
pub fn setup(program_id: &Address, name: &'static str) -> Mollusk {
    unsafe {
        std::env::set_var("SBF_OUT_DIR", "../target/deploy");
    }
    solana_logger::setup();

    Mollusk::new(program_id, name)
}
