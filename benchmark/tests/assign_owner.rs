mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
};

const PROGRAM_ID: Address = Address::from_str_const("accountowner1111111111111111111111111111111");

fn instruction(program_id: &Address, is_writable: bool) -> (Instruction, Vec<(Address, Account)>) {
    let account = Address::new_unique();
    let owner = Address::new_unique();

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
            data: owner.as_array().to_vec(),
        },
        accounts,
    )
}

#[test]
fn test_assign_owner() {
    let mollusk = setup(&PROGRAM_ID, "assign_owner");
    let (instruction, accounts) = instruction(&PROGRAM_ID, true);
    let owner = instruction.data.clone();

    let (key, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The owner should have changed to `owner`.

    assert_ne!(account.unwrap().owner.as_array(), PROGRAM_ID.as_array());
    assert_eq!(account.unwrap().owner.as_array(), owner.as_slice());
}

#[test]
fn failt_assign_owner_with_readonly_account() {
    let mollusk = setup(&PROGRAM_ID, "assign_owner");
    let (instruction, accounts) = instruction(&PROGRAM_ID, false);

    let (key, result) = run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::instruction_err(InstructionError::ModifiedProgramId)],
    );

    let account = result.get_account(&key);
    assert!(account.is_some());

    // The owner should not have changed.

    assert_eq!(account.unwrap().owner.as_array(), PROGRAM_ID.as_array());
}
