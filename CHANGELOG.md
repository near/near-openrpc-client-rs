# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/near/near-openrpc-client-rs/releases/tag/v0.1.0) - 2026-02-19

### Added

- add RpcQueryRequest constructor methods
- add NearToken and NearGas helper methods
- edition 2024, MSRV 1.88, bump reqwest to 0.13
- initial scaffold of near-openrpc-client-rs

### Fixed

- capture NEAR's extended RPC error fields (name, cause)
- *(ci)* use org-level NEARPROTOCOL_CI_PR_ACCESS token

### Other

- rename near-primitives feature to near-types
- cargo fmt for edition 2024
- *(ci)* use WarpBuild runners with Swatinem/rust-cache
- update repository URL to near org
- add CODEOWNERS for near/devex team
