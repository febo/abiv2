<h1 align="center">
  <code>abiv2</code>
</h1>

<p align="center">
  <img
    width="300"
    alt="abiv2"
    src="https://github.com/user-attachments/assets/0d47e674-0cdb-429f-a887-9fa0b9182491"
  />
</p>

<p align="center">
Rust SDK for building Solana ABIv2 programs.
</p>

## Overview

ABIv2 is the next specification describing how runtime and programs communicate
during transaction processing. It introduces a new execution interface between
Solana programs and the runtime that replaces much of the serialization,
copying, and account translation overhead present in previous ABI versions.

This repository contains an `"alpha"` (work-in-progress) SDK implementation to
write Solana programs using ABIv2. It is heavily inspired on
[pinocchio](https://github.com/anza-xyz/pinocchio), providing a similar API.

The full ABIv2 specification can be found on [SIMD-0177](https://github.com/Lichtso/solana-improvement-documents/blob/972ae0be4e930318760321c490ac2236934321e0/proposals/0177-program-runtime-abiv2.md).

> ⚠️ The content of the repository should be treated as work-in-progress and it
> has not been audited. Currently it is not possible to deploy programs using
> ABIv2. The API of the SDK is subject to change. Contributions are welcome!

### Program entrypoint

A Solana program starts its execution from an `entrypoint` function. This is
specified using the `entrypoint!` macro, which emits the required boilerplate to
set up the program entrypoint, the memory allocator and panic handler.

The `entrypoint!` macro is a convenience macro that uses three other macros to set
all required components for a program execution:

- `program_entrypoint!`: declares the program entrypoint.
- `default_allocator!`: declares the global memory allocator.
- `default_panic_handler!`: declares the handler that defines what happens when
  the program panics.

To use the `entrypoint!` macro, use the following in your entrypoint definition:

```rust
use abiv2::{
    account::Account, context::InstructionContext, entrypoint, error::ProgramError,
    ProgramResult,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _context: &InstructionContext,
    _accounts: &[Account],
    _instruction_data: &[u8],
) -> ProgramResult {
    Ok(())
}
```

All the information for the entrypoint is given by the runtime, so the entrypoint
provided is a lightweight procedure that only prepares arguments and calls the
program's `process_instruction` function.

- `InstructionContext`: contains information about the current executing
  instruction, such as the program address, whether the instruction is a
  top-level instruction or not and CPI nesting level.

- `&[Account]`: contains the information about accounts passed to the
  instruction, similarly to an `AccountInfo` (SDK) and `AccountView`
  (pinocchio).

- `&[u8]`: contains data passed to the instruction.

Another useful component is the `TransactionContext`, which provides information
about the current executing transaction. It provides access to all accounts and
instructions in the transaction and the fee payer account.

To get the `TransactionContext`:

```rust
let tx_context = TransactionContext::get();
```

To get the fee payer account as a `TransactionAccount`:

```rust
let payer = TransactionContext::payer();
```

### Syscalls

Since ABIv2 changes the way account data is passed to programs, changes to it
must be communicated via syscalls:

- `sol_assign_owner`: changes the owner of an account. This syscall is wrapped
  in the `Account::assign` helper.

- `sol_transfer_lamports`: transfers lamports from one account to another. This
  syscall is wrapped in the `Account::transfer_lamports` helper.

- `set_buffer_length`: changes the length of a memory region, i.e., account
  data. For account resizing, the syscall is wrapped in the `Account::resize`
  helper.

## How to use this repository?

The repository contains everything to start writing and testing using ABIv2.

```
+- benchmark
|    +- tests
|
+- programs
|    +- assign-owner
|    +- transfer-lamports
|
+- sdk
```

- `benchmark`: contains mollusk tests for the provided sample programs
- `programs`: contains sample programs showcasing ABIv2 functionality.
- `sdk`: contains an experimental "SDK" to write ABIv2 programs.

It is possible to add new programs into the `programs` folder, add them to cargo
workspace and then write a test for them in `benchmark/tests` to test new
functionalities.

To build all programs (necessary before running the tests):

```sh
make all
```

Tests can be run using:

```sh
make test
```

To work with the SDK in another repository, it is necessary to either use:

- [anza/agave-runtime](https://github.com/anza-xyz/agave-runtime): fork of agave
  with ABIv2 runtime support.
- [febo/mollusk](https://github.com/febo/mollusk) (branch `abiv2`): fork of
  mollusk that uses `agave-runtime`. This allows testing ABIv2 programs.

## License

The code is licensed under the [Apache License Version 2.0](./LICENSE).
