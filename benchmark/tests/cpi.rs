mod setup;

use {
    crate::setup::{run, setup, BASE_LAMPORTS},
    mollusk_svm::{
        program::{create_program_account_loader_v3, keyed_account_for_system_program},
        result::Check,
    },
    solana_account::Account,
    solana_address::Address,
    solana_instruction::{AccountMeta, Instruction},
};

const PROGRAM_ID: Address = Address::from_str_const("cpi1111111111111111111111111111111111111111");

const SYSTEM_PROGRAM_ID: Address = Address::new_from_array([0; 32]);

const TRACE_PROGRAM_ID: Address = Address::from_str_const("trace11111111111111111111111111111111111111");

const RETURN_DATA_PROGRAM_ID: Address = Address::from_str_const("returndata111111111111111111111111111111111");

fn instruction_system_program(program_id: &Address) -> (Instruction, Vec<(Address, Account)>) {
    let from = Address::new_unique();
    let to = Address::new_unique();

    let accounts = vec![
        (from, Account::new(BASE_LAMPORTS, 0, &SYSTEM_PROGRAM_ID)),
        (to, Account::new(0, 0, &SYSTEM_PROGRAM_ID)),
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
            pubkey: SYSTEM_PROGRAM_ID,
            is_signer: false,
            is_writable: false,
        },
    ];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: vec![0],
        },
        accounts,
    )
}

fn instruction_trace(program_id: &Address) -> (Instruction, Vec<(Address, Account)>) {
    let (trace_program, trace_program_account) = (
        TRACE_PROGRAM_ID,
        create_program_account_loader_v3(&TRACE_PROGRAM_ID),
    );

    let accounts = vec![(trace_program, trace_program_account)];

    let account_metas = vec![AccountMeta {
        pubkey: trace_program,
        is_signer: false,
        is_writable: false,
    }];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: vec![1],
        },
        accounts,
    )
}

fn instruction_return_data(program_id: &Address) -> (Instruction, Vec<(Address, Account)>) {
    let (return_data_program, return_data_program_account) = (
        RETURN_DATA_PROGRAM_ID,
        create_program_account_loader_v3(&RETURN_DATA_PROGRAM_ID),
    );

    let accounts = vec![(return_data_program, return_data_program_account)];

    let account_metas = vec![AccountMeta {
        pubkey: return_data_program,
        is_signer: false,
        is_writable: false,
    }];

    (
        Instruction {
            program_id: *program_id,
            accounts: account_metas,
            data: vec![2],
        },
        accounts,
    )
}

#[test]
fn test_native_cpi() {
    let mollusk = setup(&PROGRAM_ID, "cpi");
    let (instruction, accounts) = instruction_system_program(&PROGRAM_ID);

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

#[test]
fn test_abiv2_cpi() {
    let mut mollusk = setup(&PROGRAM_ID, "cpi");
    mollusk.add_program(&TRACE_PROGRAM_ID, "trace");

    let (instruction, accounts) = instruction_trace(&PROGRAM_ID);

    run(&mollusk, &instruction, &accounts, &[Check::success()]);
}

#[test]
fn test_abiv2_cpi_with_return_data() {
    let mut mollusk = setup(&PROGRAM_ID, "cpi");
    mollusk.add_program(&RETURN_DATA_PROGRAM_ID, "return_data");

    let (instruction, accounts) = instruction_return_data(&PROGRAM_ID);

    run(&mollusk, &instruction, &accounts, &[Check::success()]);
}
