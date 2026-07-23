mod setup;

use {
    crate::setup::{run, setup},
    mollusk_svm::{result::Check, sysvar::Sysvars},
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
};

const PROGRAM_ID: Address = Address::from_str_const("rentsysvar111111111111111111111111111111111");

fn instruction(program_id: &Address) -> (Instruction, Vec<(Address, Account)>) {
    let (rent_id, rent_account) = Sysvars::default().keyed_account_for_rent_sysvar();

    let account_metas = vec![AccountMeta {
        pubkey: rent_id,
        is_signer: false,
        is_writable: false,
    }];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: vec![],
        },
        vec![(rent_id, rent_account)],
    )
}

#[test]
fn test_rent_sysvar() {
    let mollusk = setup(&PROGRAM_ID, "rent_sysvar");
    let (instruction, accounts) = instruction(&PROGRAM_ID);

    run(&mollusk, &instruction, &accounts, &[Check::success()]);
}
