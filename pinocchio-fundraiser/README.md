# Pinocchio Fundraiser Program

A lightweight, zero-dependency Solana fundraiser program built using the highly-optimized **[pinocchio](https://github.com/anza-xyz/pinocchio)** framework. 

This program leverages zero-copy deserialization via **[bytemuck](https://docs.rs/bytemuck)** to achieve minimal compute unit (CU) consumption and binary footprint.

---

## Features

The program supports the complete lifecycle of a crowdfunding/fundraiser campaign:

1. **`Initialize`**: Allows a campaign creator (`maker`) to initialize a campaign by establishing a `Fundraiser` state account (PDA) and creating a vault token account (ATA) owned by that PDA.
2. **`Deposit` (Contribute)**: Allows contributors to donate tokens to the campaign. The program:
   - Verifies the campaign duration has not expired.
   - Updates the contributor's individual PDA state tracking their contributions.
   - Increments the total campaign-wide `current_amount` raised.
3. **`Claim` (Maker Claim)**: Allows the campaign creator (`maker`) to withdraw the vault tokens when the campaign target is successfully met. This instruction:
   - Performs a CPI transfer of all vault tokens to the creator's token account.
   - Closes the vault ATA.
   - Closes the `Fundraiser` state PDA to refund the rent lamports to the creator.
4. **`Refund` (Contributor Refund)**: If a campaign expires without meeting the funding target, contributors can call this instruction to:
   - Reclaim their donated tokens from the vault.
   - Close their `Contributor` state PDA to recover their rent lamports.

---

## State Accounts

### 1. `Fundraiser`
Stores the configuration and metadata of the fundraiser campaign:
*   `maker`: Creator's public key.
*   `mint_to_raise`: Mint address of the token being raised.
*   `vault`: Token account holding the campaign's deposits.
*   `amount_to_raise`: Funding target.
*   `current_amount`: Total amount contributed so far.
*   `time_started`: Unix timestamp of campaign creation.
*   `duration`: Duration of the campaign in days.
*   `bump`: PDA bump seed.

### 2. `Contributor`
Tracks individual contributions:
*   `amount`: Total tokens contributed by a specific public key.

---

## How to Build & Run Tests

### Prerequisites
Make sure you have Rust, Cargo, and the Solana CLI / SBF tools installed.

### 1. Build the On-chain Program
Compile the code into a Solana SBF (Solana Binary Format) shared library:
```bash
cargo build-sbf
```

### 2. Run the Test Suite
The project uses **[litesvm](https://github.com/d-t-a/litesvm)** to run unit and integration tests inside a sandbox environment:
```bash
cargo test
```

### Included Tests:
*   `test_create_fundraiser_instruction`: Verifies initialization of the campaign.
*   `test_donate_instruction`: Tests contributor donations and state tracking.
*   `test_claim_instruction`: Verifies that creators can claim tokens and close campaign accounts once the goal is reached.
*   `test_refund_instruction`: Verifies that contributors can reclaim their tokens and recover rent when a campaign fails and expires.
