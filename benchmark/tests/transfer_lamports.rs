mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS, PROGRAM_ID},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{error::InstructionError, AccountMeta, Instruction},
};

fn instruction(
    program_id: &Address,
    amount: u64,
    is_writable: bool,
) -> (Instruction, Vec<(Address, Account)>) {
    let from = Address::new_unique();
    let to = Address::new_unique();

    let accounts = vec![
        (from, Account::new(BASE_LAMPORTS, 0, program_id)),
        (to, Account::new(BASE_LAMPORTS, 0, program_id)),
    ];
    let account_metas = vec![
        AccountMeta {
            pubkey: from,
            is_signer: false,
            is_writable,
        },
        AccountMeta {
            pubkey: to,
            is_signer: false,
            is_writable: true,
        },
    ];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: amount.to_le_bytes().to_vec(),
        },
        accounts,
    )
}

#[test]
fn test_transfer_lamports() {
    let mollusk = setup(&PROGRAM_ID, "transfer_lamports");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 1_000_000_000, true);

    let (_, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let [(from, _), (to, _), ..] = accounts.as_slice() else {
        panic!("expected at least one account");
    };

    // Lamports must have transferred between `from` and `to`.

    let from = result.get_account(from).unwrap();
    let to = result.get_account(to).unwrap();

    assert_eq!(from.lamports, BASE_LAMPORTS - 1_000_000_000);
    assert_eq!(to.lamports, BASE_LAMPORTS + 1_000_000_000);
}

#[test]
fn fail_transfer_lamports_with_readonly_account() {
    let mollusk = setup(&PROGRAM_ID, "transfer_lamports");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 1_000_000_000, false);

    let (_, result) = run(
        &mollusk,
        &instruction,
        &accounts,
        &[Check::instruction_err(
            InstructionError::ReadonlyLamportChange,
        )],
    );

    let [(from, _), (to, _), ..] = accounts.as_slice() else {
        panic!("expected at least one account");
    };

    // No lamports should have been transferred.

    let from = result.get_account(from).unwrap();
    let to = result.get_account(to).unwrap();

    assert_eq!(from.lamports, BASE_LAMPORTS);
    assert_eq!(to.lamports, BASE_LAMPORTS);
}
