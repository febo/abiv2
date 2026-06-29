mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
};

const PROGRAM_ID: Address = Address::from_str_const("accountdata11111111111111111111111111111111");

fn instruction(
    program_id: &Address,
    value: usize,
    is_writable: bool,
) -> (Instruction, Vec<(Address, Account)>) {
    let account = Address::new_unique();

    let accounts = vec![(
        account,
        Account::new(BASE_LAMPORTS, size_of::<u64>(), program_id),
    )];
    let account_metas = vec![AccountMeta {
        pubkey: account,
        is_signer: false,
        is_writable,
    }];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: value.to_le_bytes().to_vec(),
        },
        accounts,
    )
}

#[test]
fn test_account_resize() {
    let mollusk = setup(&PROGRAM_ID, "account_data");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 2026, true);

    let (key, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The account data must the equal to `2026`.

    assert_eq!(&account.unwrap().data, 2026u64.to_le_bytes().as_slice());
}

#[test]
fn fail_account_resize_with_readonly_account() {
    let mollusk = setup(&PROGRAM_ID, "account_data");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 2026, false);

    let (key, result) = run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::instruction_err(
            InstructionError::ProgramFailedToComplete,
        )],
    );

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The account data should not have changed.

    assert_eq!(&account.unwrap().data, 0u64.to_le_bytes().as_slice());
}
