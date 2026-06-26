mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS, PROGRAM_ID},
    mollusk_svm::{program::keyed_account_for_system_program, result::Check},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
};

fn instruction(program_id: &Address) -> (Instruction, Vec<(Address, Account)>) {
    let from = Address::new_unique();
    let to = Address::new_unique();

    let accounts = vec![
        (
            from,
            Account::new(BASE_LAMPORTS, 0, &Address::from([0; 32])),
        ),
        (to, Account::new(0, 0, &Address::from([0; 32]))),
        keyed_account_for_system_program(),
    ];

    let account_metas = vec![
        AccountMeta {
            pubkey: from,
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: to,
            is_signer: true,
            is_writable: true,
        },
        AccountMeta {
            pubkey: Address::from([0; 32]),
            is_signer: false,
            is_writable: false,
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
fn test_cpi() {
    let mollusk = setup(&PROGRAM_ID, "cpi");
    let (instruction, accounts) = instruction(&PROGRAM_ID);

    let (_, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    let [(from, _), (to, _), ..] = accounts.as_slice() else {
        panic!("expected at least one account");
    };

    let from = result.get_account(from).unwrap();
    let to = result.get_account(to).unwrap();

    // The 'to' account should have lamports and owner equal to `PROGRAM_ID`.

    assert_eq!(from.lamports, BASE_LAMPORTS / 2);
    assert_eq!(to.lamports, BASE_LAMPORTS / 2);
    assert_eq!(to.data.len(), 100);
    assert_eq!(to.owner, PROGRAM_ID);
}
