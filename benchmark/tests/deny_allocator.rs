mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS, PROGRAM_ID},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
};

fn instruction(program_id: &Address, is_writable: bool) -> (Instruction, Vec<(Address, Account)>) {
    let account = Address::new_unique();

    let accounts = vec![(account, Account::new(BASE_LAMPORTS, 0, program_id))];
    let account_metas = vec![AccountMeta {
        pubkey: account,
        is_signer: false,
        is_writable,
    }];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: vec![],
        },
        accounts,
    )
}

#[test]
fn test_deny_allocator() {
    let mollusk = setup(&PROGRAM_ID, "deny_allocator");
    let (instruction, accounts) = instruction(&PROGRAM_ID, true);

    // Program should panic on allocations.
    run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::instruction_err(
            InstructionError::ProgramFailedToComplete,
        )],
    );
}
