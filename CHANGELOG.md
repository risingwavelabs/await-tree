# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-04-09

### Added
- Added `span!` macro as a replacement for `format!` with better performance ([#21](https://github.com/risingwavelabs/await-tree/pull/21))
- Added support for global registry and updated its documentation ([#17](https://github.com/risingwavelabs/await-tree/pull/17), [#18](https://github.com/risingwavelabs/await-tree/pull/18))
- Implemented `serde::Serialize` for tree to provide structured output and made it an optional feature ([#22](https://github.com/risingwavelabs/await-tree/pull/22), [#24](https://github.com/risingwavelabs/await-tree/pull/24))
- Added attributes `long_running` and `verbose` on span, removed `verbose_instrument_await` ([#20](https://github.com/risingwavelabs/await-tree/pull/20))

### Changed
- Only depend on `tokio` if spawn capability is desired ([#25](https://github.com/risingwavelabs/await-tree/pull/25))
- Added workflow for publishing crates ([#23](https://github.com/risingwavelabs/await-tree/pull/23))

### Fixed
- Fixed examples and added CI for running examples ([#19](https://github.com/risingwavelabs/await-tree/pull/19))

[0.3.0]: https://github.com/risingwavelabs/await-tree/compare/v0.2.1...v0.3.0
