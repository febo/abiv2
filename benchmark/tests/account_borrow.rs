mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
    solana_program_error::ProgramError,
};

const PROGRAM_ID: Address = Address::from_str_const("accountborrow111111111111111111111111111111");

fn instruction(program_id: &Address, duplicated: bool) -> (Instruction, Vec<(Address, Account)>) {
    let first = Address::new_unique();
    let second = if duplicated {
        first
    } else {
        Address::new_unique()
    };

    let mut accounts = vec![(
        first,
        Account::new_data(BASE_LAMPORTS, &[5; 20], program_id).unwrap(),
    )];

    if !duplicated {
        accounts.push((
            second,
            Account::new_data(BASE_LAMPORTS, &[0; 20], program_id).unwrap(),
        ));
    }

    let account_metas = vec![
        AccountMeta {
            pubkey: first,
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: second,
            is_signer: false,
            is_writable: true,
        },
    ];

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
fn test_account_borrow() {
    let mollusk = setup(&PROGRAM_ID, "account_borrow");
    let (instruction, accounts) = instruction(&PROGRAM_ID, false);

    let (_, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let [(first, _), (second, _), ..] = accounts.as_slice() else {
        panic!("expected at least one account");
    };

    // The account data must be equal.

    let first = result.get_account(first).unwrap();
    let second = result.get_account(second).unwrap();

    assert_eq!(&first.data, &second.data);
}

#[test]
fn fail_account_borrow_with_duplicated_account() {
    let mollusk = setup(&PROGRAM_ID, "account_borrow");
    let (instruction, accounts) = instruction(&PROGRAM_ID, true);

    run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::err(ProgramError::AccountBorrowFailed)],
    );
}
