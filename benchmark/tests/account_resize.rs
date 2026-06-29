mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
    solana_program_error::ProgramError,
};

const PROGRAM_ID: Address = Address::from_str_const("accountresize111111111111111111111111111111");

// Maximum account data length.
//
// https://github.com/anza-xyz/agave/blob/master/transaction-context/src/lib.rs#L19
const MAX_ACCOUNT_DATA_LEN: usize = 10 * 1024 * 1024;

fn instruction(
    program_id: &Address,
    new_len: usize,
    is_writable: bool,
) -> (Instruction, Vec<(Address, Account)>) {
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
            data: new_len.to_le_bytes().to_vec(),
        },
        accounts,
    )
}

#[test]
fn test_account_resize() {
    let mollusk = setup(&PROGRAM_ID, "account_resize");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 72, true);

    let (key, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The account data length should have increased.

    assert_eq!(account.unwrap().data.len(), 72);
}

#[test]
fn fail_account_resize_with_readonly_account() {
    let mollusk = setup(&PROGRAM_ID, "account_resize");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 72, false);

    let (key, result) = run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::instruction_err(
            InstructionError::ReadonlyDataModified,
        )],
    );

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The account data length should not have changed.

    assert_eq!(account.unwrap().data.len(), 0);
}

#[test]
fn test_account_resize_max_data_len() {
    let mollusk = setup(&PROGRAM_ID, "account_resize");
    let (instruction, accounts) = instruction(&PROGRAM_ID, MAX_ACCOUNT_DATA_LEN, true);

    let (key, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The account data length should have increased to `MAX_ACCOUNT_DATA_LEN`.

    assert_eq!(account.unwrap().data.len(), MAX_ACCOUNT_DATA_LEN);
}

#[test]
fn fail_account_resize_over_max_data_len() {
    let mollusk = setup(&PROGRAM_ID, "account_resize");
    // Going 1 byte over the limit.
    let (instruction, accounts) = instruction(&PROGRAM_ID, MAX_ACCOUNT_DATA_LEN + 1, true);

    run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::err(ProgramError::InvalidRealloc)],
    );
}
