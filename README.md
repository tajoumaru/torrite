# *torrite* is a blazing-fast CLI-tool for creating BitTorrent v1 & v2 metainfo files, written in Rust

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

| Tool | 1. Large ISO (5GB) | 2. Source Tree (Nested Tiny) | 3. User Docs (Mixed) | 4. Assets (Large Files) | 5. Edge Cases (Boundaries) | 6. Metadata Bomb (10k files) | Average&nbsp;↑ |
|---|---|---|---|---|---|---|---|
| **torrite (V2 Only)** | **0.162s** | **0.022s** | **0.060s** | **0.140s** | **0.011s** | **0.063s** | **0.076s** |
| **torrite (V1)** | **0.149s** | **0.024s** | **0.076s** | **0.141s** | **0.015s** | **0.076s** | **0.080s** |
| mkbrr (V1) | 0.173s | 0.145s | 0.142s | 0.159s | 0.012s | 0.237s | 0.145s |
| **torrite (Hybrid)** | **0.304s** | **0.106s** | **0.131s** | **0.270s** | **0.020s** | **0.259s** | **0.182s** |
| torrenttools (V1) | 0.684s | 0.354s | 0.543s | 0.880s | 0.333s | 0.636s | 0.572s |
| imdl (V1) | 5.191s | 0.042s | 1.626s | 4.651s | 0.029s | 0.086s | 1.938s |
| mktorrent (V1) | 5.303s | 0.039s | 1.661s | 4.776s | 0.029s | 0.078s | 1.981s |
| torrenttools (V2) | 1.014s | 2.166s | 0.952s | 1.042s | 0.336s | 10.411s | 2.653s |
| torrenttools (Hybrid) | 1.209s | 2.615s | 1.181s | 1.190s | 0.354s | 12.920s | 3.245s |
> *Benchmarks performed on AMD Ryzen 9 7950X3D, 32GB DDR5-6000 RAM, Gen4 NVMe SSD*\
> *Test file generator can be found in `src/bin/generate_bench_data.rs`*

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
