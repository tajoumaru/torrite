# *torrite* is a blazing-fast CLI-tool for creating BitTorrent v1 & v2 metainfo files, written in Rust

<p align="center">
  <img width="180" height="180" alt="torrite" src="https://github.com/user-attachments/assets/d581dde1-a765-43b5-aa58-5ada34451ea9" />
</p>

Named after *ferrite* (iron oxide), keeping true to the metal-themed Rust naming tradition, which is also used to make magnets.

## Features

- **Full [mktorrent](https://github.com/pobrn/mktorrent) compatibility** — All command-line flags from mktorrent are supported and work identically
- **BitTorrent v2 support** — Create modern v2-only or hybrid (v1+v2) torrents with `--v2` and `--hybrid` flags
- **Interactive TUI mode** — User-friendly terminal interface for creating and editing torrents (powered by ratatui)
- **Blazing fast performance** — See benchmarks below for real-world speed comparisons
- **Multi-threaded hashing** — Utilizes all CPU cores by default for maximum throughput
- **Verification & Editing** — Verify local files against metadata or edit existing torrent files
- **Configuration & Profiles** — Save defaults and use profiles for specific trackers

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

Torrite uses subcommands for different operations. The default subcommand is `create`, so you can use it just like `mktorrent`.

### Interactive Mode

Torrite features an interactive terminal UI for easier torrent creation and editing:

```bash
# Launch interactive creation wizard
torrite
# or
torrite create

# Launch interactive editor
torrite edit my-torrent.torrent
```

The interactive mode provides a user-friendly interface with guided prompts for all torrent settings.

### Create a torrent (Default)

```bash
# Basic usage (defaults to 'create')
torrite -a http://tracker.example.com:8080/announce my-file.iso

# Explicit subcommand
torrite create -a http://tracker.example.com/announce my-data/

# Use a specific profile (e.g., PTP, GGn) defined in config
torrite -P PTP -a http://tracker.example.com/announce my-movie.mkv
```

### Verify a torrent

```bash
torrite verify --path /path/to/downloaded/files my-torrent.torrent
```

### Edit a torrent

```bash
# Interactive mode (no flags)
torrite edit my-torrent.torrent

# CLI mode - Change the announce URL
torrite edit --replace-announce http://new.tracker.com/announce my-torrent.torrent

# Make it private
torrite edit --private my-torrent.torrent
```

### Inspect metadata

```bash
torrite inspect my-torrent.torrent
```

## Command-line Options

```
Usage: torrite [OPTIONS] <COMMAND>

Commands:
  create   Create a new torrent (default)
  verify   Verify local files against a torrent
  inspect  Inspect a torrent file's metadata
  edit     Edit an existing torrent's metadata
  help     Print this message or the help of the given subcommand(s)

Options:
      --config <FILE>  Path to a custom configuration file
  -h, --help           Print help
  -V, --version        Print version
```

### Create Options

```
Usage: torrite create [OPTIONS] <TARGET>

Arguments:
  <TARGET>  The file or directory to create a torrent from

Options:
      --config <FILE>      Path to a custom configuration file
  -P, --profile <PROFILE>  Profile to use from configuration
  -a, --announce <URL>     Announce URL(s) - can be specified multiple times
  -c, --comment <COMMENT>  Add a comment to the metainfo
  -d, --no-date            Don't write the creation date
  -e, --exclude <PATTERN>  Exclude files matching pattern (glob)
  -f, --force              Overwrite output file if it exists
  -l, --piece-length <N>   Set the piece length to 2^N bytes (e.g., 18 for 256KB)
  -n, --name <NAME>        Set the name of the torrent
  -o, --output <FILE>      Set the output file path
      --date <TIMESTAMP>   Set the creation date (Unix timestamp)
  -p, --private            Set the private flag
  -s, --source <SOURCE>    Add source string embedded in infohash
  -t, --threads <N>        Number of threads for hashing
  -v, --verbose            Verbose output
  -w, --web-seed <URL>     Web seed URL(s)
  -x, --cross-seed         Ensure info hash is unique for easier cross-seeding
      --info-hash          Display the info hash of the created torrent
      --json               Output results in JSON format
      --v2                 Create a v2-only torrent (no v1 compatibility)
      --hybrid             Create a hybrid torrent (v1 + v2 compatibility)
      --dry-run            Calculate piece length and show info without hashing
```

## Configuration & Profiles

Torrite supports configuration via TOML files. It looks for config in:
1. `--config` CLI argument
2. `TORRITE_CONFIG` environment variable
3. `torrite.toml` in current directory
4. `~/.config/torrite/config.toml` (or equivalent on your OS)

Example config:

```toml
[default]
piece_length = 19
announce = ["http://my.default.tracker/announce"]

[profiles.PTP]
source = "PTP"
piece_length = 20
```

Use profiles with `-P`: `torrite -P PTP ...`

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
