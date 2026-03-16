# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/near/near-openrpc-client-rs/compare/v0.1.0...v0.2.0) - 2026-03-12

### Fixed

- handle legacy nearcore error responses for query methods ([#17](https://github.com/near/near-openrpc-client-rs/pull/17))
- fall back to master when openrpc.json missing from latest release ([#16](https://github.com/near/near-openrpc-client-rs/pull/16))

### Other

- sync openrpc.json and regenerate types ([#13](https://github.com/near/near-openrpc-client-rs/pull/13))
- sync openrpc.json from latest nearcore release instead of master ([#14](https://github.com/near/near-openrpc-client-rs/pull/14))

## [0.1.0](https://github.com/near/near-openrpc-client-rs/releases/tag/v0.1.0) - 2026-03-11

### Added

- add errors module with per-method RPC error enums ([#9](https://github.com/near/near-openrpc-client-rs/pull/9))
- update mainnet example with dedicated query endpoints
- add client methods for dedicated query endpoints
- update spec with dedicated EXPERIMENTAL query endpoints
- edition 2024, MSRV 1.88, bump reqwest to 0.13
- initial scaffold of near-openrpc-client-rs

### Fixed

- capture NEAR's extended RPC error fields (name, cause)
- *(ci)* use org-level NEARPROTOCOL_CI_PR_ACCESS token

### Other

- update examples to use dedicated query endpoints
- cargo fmt for edition 2024
- *(ci)* use WarpBuild runners with Swatinem/rust-cache
- update repository URL to near org
- add CODEOWNERS for near/devex team
