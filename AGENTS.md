# Agent Context for Torrite

This file provides context for AI agents working on the `torrite` project.

## Project Overview

**torrite** is a high-performance CLI tool for creating BitTorrent v1 & v2 metainfo files, written in Rust.
It aims for full command-line compatibility with `mktorrent` while offering significant speed improvements and modern BitTorrent features (v2 and hybrid torrents).

## Key Features & Goals

1.  **mktorrent Compatibility**: Must support all flags from `mktorrent` and behave identically where possible.
2.  **Performance**: The tool is designed to be "blazing fast", utilizing multi-threading and optimized I/O.
3.  **BitTorrent v2**: Support for v2-only and hybrid (v1 + v2) torrent creation.

## Codebase Structure

The project is a standard Rust binary crate with a library structure.

-   `src/bin/`: Contains the main binary (`main.rs`) and benchmark tools.
    -   `main.rs`: Entry point, parses CLI args and invokes the library.
    -   `generate_bench_data.rs`: Tool to generate test files for benchmarking.
    -   `run_benchmarks.rs`: Tool to run performance benchmarks.
-   `src/lib.rs`: The core library logic.
-   `src/builder.rs`: Logic for building the torrent metadata (`TorrentBuilder`).
-   `src/cli.rs`: CLI argument parsing using `clap`.
-   `src/config.rs`: Configuration handling.
-   `src/hashing/`: Hashing logic (SHA-1 for v1, SHA-256 for v2, parallel processing).
-   `src/models/`: Data structures for torrent files (`Torrent`, `File`, etc.).
-   `src/piece.rs`: Piece calculation and management.
-   `src/scanner.rs`: File system scanning logic (`jwalk`).
-   `src/tree.rs`: Merkle tree implementation for v2 torrents.

## Development Guidelines

-   **Dependencies**: The project uses `clap` for CLI, `serde` + `serde_bencode` for serialization, `sha1`/`sha2` for hashing, and `rayon` for parallelism.
-   **Testing**:
    -   Run tests via `cargo test`.
    -   Performance regressions should be checked using the benchmark tools in `src/bin/`.
-   **Style**: Follow standard Rust idioms (rustfmt, clippy).

## Common Commands

-   **Build**: `cargo build --release`
-   **Run**: `cargo run -- <args>`
-   **Test**: `cargo test`
-   **Generate Bench Data**: `cargo run --features dev --bin generate_bench_data`
-   **Run Benchmarks**: `cargo run --features dev --bin run_benchmarks`

## Important Considerations

-   When modifying CLI arguments, ensure backward compatibility with `mktorrent` flags.
-   Performance is critical. Avoid blocking I/O on the main thread where possible and prefer parallel iterators for heavy computations.
-   Pay attention to `v1`, `v2`, and `hybrid` modes. Changes in hashing logic must support all three configurations correctly.
