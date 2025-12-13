# *torrite* is a blazing-fast CLI-tool for creating BitTorrent metainfo files

<p align="center">
  <img width="180" height="180" alt="torrite" src="https://github.com/user-attachments/assets/d581dde1-a765-43b5-aa58-5ada34451ea9" />
</p>

Named after *ferrite* (iron oxide), keeping true to the metal-themed Rust naming tradition, which are also used to make magnets.

## Features

- **Full [mktorrent](https://github.com/pobrn/mktorrent) compatibility** — All command-line flags from mktorrent are supported and work identically
- **BitTorrent v2 support** — Create modern v2-only or hybrid (v1+v2) torrents with `--v2` and `--hybrid` flags
- **Blazing fast performance** — See benchmarks below for real-world speed comparisons
- **Multi-threaded hashing** — Utilizes all CPU cores by default for maximum throughput

## Performance

| Tool | 1. Large ISO (5GB) | 2. Source Tree (Nested Tiny) | 3. User Docs (Mixed) | 4. Assets (Large Files) | 5. Edge Cases (Boundaries) | 6. Metadata Bomb (10k files) |
|---|---|---|---|---|---|---|
| **torrite (V1)** | **0.153s** | **0.025s** | **0.075s** | **0.186s** | **0.015s** | **0.076s** |
| **torrite (V2 Only)** | **0.161s** | **0.024s** | **0.060s** | **0.194s** | **0.012s** | **0.067s** |
| **torrite (Hybrid)** | **0.308s** | **0.109s** | **0.130s** | **0.370s** | **0.020s** | **0.264s**\* |
| [mktorrent](https://github.com/pobrn/mktorrent) (V1) | 5.274s | 0.040s | 1.556s | 6.563s | 0.027s | 0.075s |
> \* Hybrid mode is significantly slower on 10k empty files due to necessary padding file generation for each piece. (BEP 52 & BEP 47)

*Benchmarks performed on AMD Ryzen 9 7950X3D, 32GB DDR5-6000 RAM, Gen4 NVMe SSD*\
*Benchmark file generator can be found in `src/bin/generate_bench_data.rs`*

**Speed improvements over mktorrent:**
- Large Single File (5GB) **34.5x faster**
- Deeply nested repo tree (~1500 × 1KB-20KB): **1.6x faster**
- Mixed docs (500 × 15KB-100MB): **20.7x faster**
- Large asset files (20 × 50MB-500MB): **35.3x faster**
- Piece boundary files (1 byte above or below common piece boundaries + primes): **1.8x faster**
- Metadata bomb (10k × 0 or 1 Byte): **Identical performance**

## Installation

### From crates.io

```bash
cargo install torrite
```

### From source

```bash
git clone https://github.com/tajoumaru/torrite.git
cd torrite
cargo build --release
# Binary will be in target/release/torrite
```

## Usage

### Basic usage

Create a torrent with a tracker URL:

```bash
torrite -a http://tracker.example.com:8080/announce my-file.iso
```

### Create a v2-only torrent

```bash
torrite --v2 -a http://tracker.example.com:8080/announce my-data/
```

### Create a hybrid torrent (v1 + v2)

```bash
torrite --hybrid -a http://tracker.example.com:8080/announce my-data/
```

### Advanced usage

```bash
# Private torrent with custom piece length and comment
torrite -a http://tracker.example.com/announce \
  -p \
  -l 20 \
  -c "My awesome torrent" \
  -o output.torrent \
  my-directory/

# Multiple trackers for redundancy
torrite -a http://tracker1.example.com/announce \
  -a http://tracker2.example.com/announce \
  -a udp://tracker3.example.com:6969/announce \
  my-file.tar.gz

# Exclude unwanted files
torrite -a http://tracker.example.com/announce \
  -e "*.DS_Store,*.tmp,Thumbs.db" \
  my-project/
```

## Command-line Options

```
Usage: torrite [OPTIONS] <TARGET>

Arguments:
  <TARGET>  The file or directory to create a torrent from

Options:
  -a, --announce <URL>         Announce URL(s) - can be specified multiple times for backup trackers
  -c, --comment <COMMENT>      Add a comment to the metainfo
  -d, --no-date                Don't write the creation date
  -e, --exclude <PATTERN>      Exclude files matching pattern (glob) - can be comma-separated
  -f, --force                  Overwrite output file if it exists
  -l, --piece-length <N>       Set the piece length to 2^N bytes (e.g., 18 for 256KB)
  -n, --name <NAME>            Set the name of the torrent (defaults to basename of target)
  -o, --output <FILE>          Set the output file path (defaults to <name>.torrent)
  -p, --private                Set the private flag
  -s, --source <SOURCE>        Add source string embedded in infohash
  -t, --threads <N>            Number of threads for hashing (defaults to number of CPU cores)
  -v, --verbose                Verbose output
  -w, --web-seed <URL>         Web seed URL(s) - can be specified multiple times
  -x, --cross-seed             Ensure info hash is unique for easier cross-seeding
      --v2                     Create a v2-only torrent (no v1 compatibility)
      --hybrid                 Create a hybrid torrent (v1 + v2 compatibility)
  -h, --help                   Print help
  -V, --version                Print version
```

## BitTorrent v2 Support

torrite extends mktorrent by supporting the modern BitTorrent v2 specification:

- **`--v2`** — Creates v2-only torrents using SHA-256 merkle trees. These torrents are not compatible with v1-only clients.
- **`--hybrid`** — Creates hybrid torrents that work with both v1 and v2 clients, allowing gradual ecosystem transition.

Note: Hybrid torrents take longer to generate as they compute both v1 (SHA-1) and v2 (SHA-256) hashes.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

This project is a complete rewrite and is not affiliated with the original mktorrent project, though it maintains command-line compatibility.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
