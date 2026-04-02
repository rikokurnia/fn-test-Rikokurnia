# Solana Crowdfunding Platform

A decentralized crowdfunding smart contract built on Solana using the Anchor framework. This platform allows creators to raise funds securely through Program Derived Addresses (PDAs), guaranteeing that funds are only accessible if the campaign hits its goal by the deadline.

## Core Features

*   **Secure Campaigns**: Create campaigns with a target goal (SOL) and a deadline (Unix timestamp).
*   **PDA Vaults**: Funds are locked in a secure Vault PDA controlled only by the smart contract.
*   **Trustless Payouts**: Creators can only withdraw funds if the goal is met and the deadline has passed.
*   **Automated Refunds**: If a campaign fails to hit its goal, donors can claim their exact contribution back from the vault.

## Project Structure

*   `programs/solana_crowdfunding/src/lib.rs`: The Anchor smart contract logic.
*   `tests/solana_crowdfunding.ts`: Comprehensive TypeScript test suite covering success, failure, and edge cases.
*   `Anchor.toml`: Program configuration and deployment settings.

## Prerequisites

- [Solana Tool Suite](https://docs.solana.com/cli/install-solana-cli-tools)
- [Rust](https://www.rust-lang.org/tools/install)
- [Anchor Framework](https://www.anchor-lang.com/docs/installation)
- [Node.js & Yarn](https://yarnpkg.com/getting-started/install)

## Getting Started

1.  **Install dependencies**:
    ```bash
    yarn install
    ```

2.  **Build the program**:
    ```bash
    anchor build
    ```

3.  **Run tests**:
    ```bash
    anchor test
    ```

## Development and Deployment

The program is designed to be deployed to Solana Devnet. 

**Program ID**: `4g55JHQDi9diLma9XsAwhdBNSkuEVBK9vNExEMPmcUTK`

To deploy:
```bash
anchor deploy
```

## Security Design

- **Separate Vaults**: Each campaign has its own unique Vault PDA (`["vault", campaign_pubkey]`), ensuring funds are isolated.
- **Explicit Checks**: Every instruction verifies the campaign state (deadline, raised amount, claimed status) before executing transfers.
- **Double-Refund Guard**: Receipt accounts track individual contributions and are set to zero immediately upon refund to prevent double claims.

---
Created for the Solana Crowdfunding Challenge.
