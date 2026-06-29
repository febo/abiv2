mod setup;

use {
    crate::setup::{run, setup},
    mollusk_svm::result::Check,
    solana_account::Account,
    solana_address::Address,
    solana_instruction::Instruction,
};

const PROGRAM_ID: Address = Address::from_str_const("returndata111111111111111111111111111111111");

fn instruction(program_id: &Address, address: &Address) -> (Instruction, Vec<(Address, Account)>) {
    (
        Instruction {
            program_id: *program_id,
            accounts: vec![],
            data: address.as_array().to_vec(),
        },
        // Dummy account to satisfy the `run` helper.
        vec![(Address::new_unique(), Account::default())],
    )
}

#[test]
fn test_return_data() {
    let mollusk = setup(&PROGRAM_ID, "return_data");

    let address = Address::new_unique();
    let (instruction, accounts) = instruction(&PROGRAM_ID, &address);

    let (_, result) = run(&mollusk, &instruction, &accounts, &[Check::success()]);

    assert_eq!(&result.return_data, address.as_array());
}
