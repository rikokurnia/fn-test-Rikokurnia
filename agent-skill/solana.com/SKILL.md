# Solana Agent Skills

> Foundation-maintained skills for AI agents building on Solana.
> Install any skill: `npx skills add <url>`

## Install All Skills

```
npx skills add https://github.com/solana-foundation/solana-dev-skill
```

## Skills

### Common Errors & Solutions
Diagnose and fix common errors encountered when building on Solana, including GLIBC issues, Anchor version conflicts, and RPC errors.
- **Category**: Reference

### Version Compatibility Matrix
Reference table for matching Anchor, Solana CLI, Rust, and Node.js versions to avoid toolchain conflicts.
- **Category**: Tooling

### Confidential Transfers
Implement private, encrypted token balances on Solana using the Token-2022 confidential transfers extension.
- **Category**: Tokens

### Frontend with framework-kit
Build React and Next.js Solana apps with a single client instance, Wallet Standard-first connection, and minimal client-side footprint.
- **Category**: Frontend

### IDL & Client Code Generation
Generate type-safe program clients from IDLs using Codama, eliminating hand-maintained serializers across languages.
- **Category**: Tooling

### Kit ↔ web3.js Interop
Patterns for bridging @solana/kit and legacy @solana/web3.js at adapter boundaries while migrating incrementally.
- **Category**: Tooling

### Payments & Commerce
Build checkout flows, payment buttons, and QR-based payment requests using Commerce Kit and Solana Pay.
- **Category**: Payments

### Curated Resources
Authoritative Solana learning platforms, documentation, tooling references, and community resources.
- **Category**: Reference

### Security Checklist
Program and client security checklist covering account validation, signer checks, and common attack vectors to review before deploying.
- **Category**: Security

### Testing Strategy
A testing pyramid for Solana programs using LiteSVM for fast unit tests, Mollusk for isolated instruction checks, and Surfpool for integration tests with realistic state.
- **Category**: Testing
