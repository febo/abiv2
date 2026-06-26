mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS, PROGRAM_ID},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
};

fn instruction(
    program_id: &Address,
    accounts_len: usize,
) -> (Instruction, Vec<(Address, Account)>) {
    let accounts: Vec<(Address, Account)> = (0..accounts_len)
        .map(|_| {
            (
                Address::new_unique(),
                Account::new(BASE_LAMPORTS, 0, program_id),
            )
        })
        .collect();

    let account_metas = accounts
        .iter()
        .map(|(address, _)| AccountMeta {
            pubkey: *address,
            is_signer: false,
            is_writable: false,
        })
        .collect();

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
fn test_trace() {
    let mollusk = setup(&PROGRAM_ID, "trace");
    let (instruction, accounts) = instruction(&PROGRAM_ID, 64);

    run(&mollusk, &instruction, &accounts, &[Check::success()]);
}
