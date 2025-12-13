use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, exit};

// --- CONFIGURATION ---

// 1. The name or path of YOUR Rust tool
const torrite_BIN: &str = "target/release/torrite";

// 2. The name or path of the LEGACY tool (mktorrent)
//    Ensure mktorrent is in your PATH or put the full path here.
const MKTORRENT_BIN: &str = "mktorrent";

// 3. Command templates.
//    {INPUT} will be replaced by the file/folder path.
//    {OUTPUT} will be replaced by the output torrent path.
//
//    NOTE: We add a dummy announce URL because mktorrent often requires one.
const torrite_CMD: &str = "{BIN} {INPUT} -a http://localhost/announce -l 21 -f -o {OUTPUT}";
const torrite_HYBRID_CMD: &str =
    "{BIN} {INPUT} -a http://localhost/announce -l 21 -f --hybrid -o {OUTPUT}";
const torrite_V2_ONLY_CMD: &str =
    "{BIN} {INPUT} -a http://localhost/announce -l 21 -f --v2 -o {OUTPUT}";
const MKTORRENT_CMD: &str = "{BIN} -a http://localhost/announce -l 21 -f -o {OUTPUT} {INPUT}";

struct BenchmarkCase {
    name: &'static str,
    path: &'static str,
}

#[derive(Deserialize)]
struct MinimalTorrent {
    info: serde_bencode::value::Value,
}

#[derive(Deserialize, Debug)]
struct HyperfineOutput {
    results: Vec<HyperfineRun>,
}

#[derive(Deserialize, Debug)]
struct HyperfineRun {
    command: String,
    mean: f64,
}

fn calculate_info_hash(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let torrent: MinimalTorrent =
        serde_bencode::from_bytes(&data).map_err(|e| format!("Failed to decode bencode: {}", e))?;

    // We re-encode the info dictionary to calculate the hash.
    // This relies on serde_bencode producing canonical bencode (sorted keys),
    // which is required by the spec and implemented by the crate.
    let info_bytes = serde_bencode::to_bytes(&torrent.info)
        .map_err(|e| format!("Failed to re-encode info: {}", e))?;

    let mut hasher = Sha1::new();
    hasher.update(&info_bytes);
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

fn main() {
    // 1. Check dependencies
    check_binary_exists("hyperfine");

    // allow user to override binary paths via env vars
    let torrite = env::var("torrite").unwrap_or_else(|_| torrite_BIN.to_string());
    let mktorrent = env::var("MKTORRENT").unwrap_or_else(|_| MKTORRENT_BIN.to_string());

    // Check for debug flag
    let args: Vec<String> = env::args().collect();
    let debug_mode = args.contains(&"--debug".to_string());

    check_binary_exists(&torrite);
    check_binary_exists(&mktorrent);

    println!("---------------------------------------------------");
    println!("🚀 Torrent Benchmark Runner");
    println!("   Candidate: {}", torrite);
    println!("   Baseline:  {}", mktorrent);
    if debug_mode {
        println!("   Debug Mode: ON (Results will be kept)");
    }
    println!("---------------------------------------------------");

    let root = Path::new("benchmark_data");
    if !root.exists() {
        eprintln!("❌ Error: 'benchmark_data' folder not found.");
        eprintln!(
            "   Please run the data generation script first: cargo run --bin generate_bench_data --features dev"
        );
        exit(1);
    }

    // 2. Define Scenarios
    //    We map the descriptive name to the path inside benchmark_data
    let scenarios = vec![
        BenchmarkCase {
            name: "1. Large ISO (5GB)",
            path: "distro_images/huge_distro.iso",
        },
        BenchmarkCase {
            name: "2. Source Tree (Nested Tiny)",
            path: "src_tree",
        },
        BenchmarkCase {
            name: "3. User Docs (Mixed)",
            path: "user_documents",
        },
        BenchmarkCase {
            name: "4. Assets (Large Files)",
            path: "assets",
        },
        BenchmarkCase {
            name: "5. Edge Cases (Boundaries)",
            path: "edge_cases",
        },
        BenchmarkCase {
            name: "6. Metadata Bomb (10k files)",
            path: "swarm_stress",
        },
    ];

    let results_dir = Path::new("benchmark_results");
    if results_dir.exists() {
        fs::remove_dir_all(results_dir).expect("Failed to clean previous results");
    }
    fs::create_dir_all(results_dir).expect("Failed to create results dir");

    // Store results: Tool Name -> Vec<Mean Time String>
    // We use BTreeMap to keep keys sorted, but we might want specific order.
    // Let's use a wrapper or just manage insertion order.
    // Actually, we can just use a predefined list of tool keys to ensure row order.
    let tool_names = vec![
        "mktorrent (V1)",
        "torrite (V1)",
        "torrite (V2 Only)",
        "torrite (Hybrid)",
    ];

    let mut aggregated_results: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for tool in &tool_names {
        aggregated_results.insert(tool.to_string(), Vec::new());
    }

    // 3. Run Hyperfine for each scenario
    for case in &scenarios {
        let input_path = root.join(case.path);

        if !input_path.exists() {
            println!(
                "⚠️ Skipping '{}': Path not found {:?}",
                case.name, input_path
            );
            // Add "N/A" for missing scenarios to keep alignment
            for tool in &tool_names {
                aggregated_results
                    .get_mut(*tool)
                    .unwrap()
                    .push("N/A".to_string());
            }
            continue;
        }

        println!("\n▶️  Running Scenario: {}", case.name);

        // Construct Output Paths (unique per scenario to allow verification)
        let safe_name = sanitize_filename(case.name);
        let out_torrite = results_dir.join(format!("{}_torrite.torrent", safe_name));
        let out_torrite_hybrid = results_dir.join(format!("{}_torrite_hybrid.torrent", safe_name));
        let out_torrite_v2 = results_dir.join(format!("{}_torrite_v2.torrent", safe_name));
        let out_mktorrent = results_dir.join(format!("{}_mktorrent.torrent", safe_name));

        // Format Commands
        let cmd_torrite = format_command(torrite_CMD, &torrite, &input_path, &out_torrite);
        let cmd_torrite_hybrid = format_command(
            torrite_HYBRID_CMD,
            &torrite,
            &input_path,
            &out_torrite_hybrid,
        );
        let cmd_torrite_v2 =
            format_command(torrite_V2_ONLY_CMD, &torrite, &input_path, &out_torrite_v2);
        let cmd_mktorrent = format_command(MKTORRENT_CMD, &mktorrent, &input_path, &out_mktorrent);

        let json_output_path = results_dir.join(format!("result_{}.json", safe_name));

        // Execute Hyperfine
        let status = Command::new("hyperfine")
            .arg("--warmup")
            .arg("1")
            .arg("--min-runs")
            .arg("3")
            .arg("--export-json")
            .arg(&json_output_path)
            .arg("-n")
            .arg("torrite (V1)")
            .arg(&cmd_torrite)
            .arg("-n")
            .arg("torrite (Hybrid)")
            .arg(&cmd_torrite_hybrid)
            .arg("-n")
            .arg("torrite (V2 Only)")
            .arg(&cmd_torrite_v2)
            .arg("-n")
            .arg("mktorrent (V1)")
            .arg(&cmd_mktorrent)
            .status()
            .expect("Failed to run hyperfine");

        if !status.success() {
            eprintln!("❌ Hyperfine reported an error for scenario: {}", case.name);
            for tool in &tool_names {
                aggregated_results
                    .get_mut(*tool)
                    .unwrap()
                    .push("Err".to_string());
            }
        } else {
            // Read JSON results
            let json_content =
                fs::read_to_string(&json_output_path).expect("Failed to read hyperfine json");
            let output: HyperfineOutput =
                serde_json::from_str(&json_content).expect("Failed to parse hyperfine json");

            // Create a map for this run to easily loop by name
            let mut run_map: BTreeMap<String, f64> = BTreeMap::new();
            for res in output.results {
                run_map.insert(res.command, res.mean);
            }

            // Populate aggregated results preserving order
            for tool in &tool_names {
                if let Some(mean) = run_map.get(*tool) {
                    aggregated_results
                        .get_mut(*tool)
                        .unwrap()
                        .push(format!("{:.3}s", mean));
                } else {
                    aggregated_results
                        .get_mut(*tool)
                        .unwrap()
                        .push("Missing".to_string());
                }
            }

            // Verify Output
            println!("   Verifying output correctness...");
            match (
                calculate_info_hash(&out_torrite),
                calculate_info_hash(&out_mktorrent),
            ) {
                (Ok(hash_torrite), Ok(hash_mktorrent)) => {
                    if hash_torrite == hash_mktorrent {
                        println!("   ✅ Info Hashes Match: {}", hash_torrite);
                    } else {
                        println!("   ❌ Info Hash Mismatch!");
                        println!("      torrite:   {}", hash_torrite);
                        println!("      mktorrent: {}", hash_mktorrent);
                    }
                }
                (Err(e), _) => {
                    println!(
                        "   ⚠️ Could not verify: Error reading torrite output: {}",
                        e
                    )
                }
                (_, Err(e)) => {
                    println!(
                        "   ⚠️ Could not verify: Error reading mktorrent output: {}",
                        e
                    )
                }
            }

            // Check Hybrid generation success
            match calculate_info_hash(&out_torrite_hybrid) {
                Ok(hash) => println!("   ✅ Hybrid Torrent Generated: {}", hash),
                Err(e) => println!("   ❌ Hybrid Torrent Error: {}", e),
            }

            // Check V2 generation success
            match calculate_info_hash(&out_torrite_v2) {
                Ok(hash) => println!("   ✅ V2 Torrent Generated: {}", hash),
                Err(e) => println!("   ❌ V2 Torrent Error: {}", e),
            }
        }
    }

    // --- Generate Markdown Table ---
    println!("\n📊 Benchmark Summary\n");

    // Header
    print!("| Tool |");
    for case in &scenarios {
        print!(" {} |", case.name);
    }
    println!();

    // Separator
    print!("|---|");
    for _ in &scenarios {
        print!("---|");
    }
    println!();

    // Rows
    for tool in &tool_names {
        print!("| {} |", tool);
        if let Some(times) = aggregated_results.get(*tool) {
            for time in times {
                print!(" {} |", time);
            }
        }
        println!();
    }
    println!();

    // Cleanup
    if !debug_mode {
        if let Err(e) = fs::remove_dir_all(results_dir) {
            eprintln!("⚠️ Warning: Failed to clean up results directory: {}", e);
        } else {
            println!("🧹 Cleaned up benchmark results.");
        }
    } else {
        println!("📝 Results kept in '{}'", results_dir.display());
    }

    println!("\n✅ Benchmarks Complete.");
}

// Helper to check if a binary is runnable
fn check_binary_exists(bin: &str) {
    // Simple check: try running with --help or --version
    let status = Command::new("which").arg(bin).output(); // 'which' works on unix, for windows use 'where'

    // Fallback logic if 'which' fails or isn't present,
    // we just try to spawn the command itself with --version
    if status.is_err() || !status.unwrap().status.success() {
        // If it's a path like ./target/release/..., check file existence
        if bin.contains("/") || bin.contains("\\") {
            if !Path::new(bin).exists() {
                // If it is our tool, we can try to be helpful
                if bin.contains("target/release") {
                    eprintln!("❌ Binary not found at path: {}", bin);
                    eprintln!("   Hint: Did you run 'cargo build --release'?");
                } else {
                    eprintln!("❌ Critical: Binary not found at path: {}", bin);
                }
                exit(1);
            }
        } else {
            // It's a command in PATH, but we can't find it.
            // Just warn.
            println!(
                "⚠️  Warning: Could not confirm location of '{}'. Assuming it is in PATH.",
                bin
            );
        }
    }
}

// Replace placeholders in command template
fn format_command(template: &str, bin: &str, input: &Path, output: &Path) -> String {
    template
        .replace("{BIN}", bin)
        .replace("{INPUT}", input.to_str().unwrap())
        .replace("{OUTPUT}", output.to_str().unwrap())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(" ", "_")
        .replace(".", "")
        .replace("(", "")
        .replace(")", "")
        .to_lowercase()
}
