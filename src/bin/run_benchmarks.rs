use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio, exit};

// --- CONFIGURATION ---

const TORRITE_BIN: &str = "target/release/torrite";
const TORRITE_CMD: &str = "{BIN} {INPUT} -l 23 -o {OUTPUT}";
const TORRITE_HYBRID_CMD: &str = "{BIN} {INPUT} -l 23 --hybrid -o {OUTPUT}";
const TORRITE_V2_ONLY_CMD: &str = "{BIN} {INPUT} -l 23 --v2 -o {OUTPUT}";

const MKTORRENT_BIN: &str = "mktorrent";
const MKBRR_BIN: &str = "mkbrr";
const TORRENTTOOLS_BIN: &str = "torrenttools";
const IMDL_BIN: &str = "imdl";
const TORF_BIN: &str = "torf";
const MKTORRENT_CMD: &str = "{BIN} -l 23 -o {OUTPUT} {INPUT}";
const MKBRR_CMD: &str = "{BIN} create {INPUT} -l 23 -o {OUTPUT}";
const TORRENTTOOLS_V1_CMD: &str = "{BIN} create -v 1 -l 23 -o {OUTPUT} {INPUT}";
const TORRENTTOOLS_V2_CMD: &str = "{BIN} create -v 2 -l 23 -o {OUTPUT} {INPUT}";
const TORRENTTOOLS_HYBRID_CMD: &str = "{BIN} create -v hybrid -l 23 -o {OUTPUT} {INPUT}";
const IMDL_CMD: &str = "{BIN} torrent create -p 8mib -o {OUTPUT} {INPUT}";
const TORF_CMD: &str = "{BIN} {INPUT} -o {OUTPUT}";

struct BenchmarkCase {
    name: &'static str,
    path: &'static str,
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

#[derive(Serialize, Debug)]
struct BenchmarkResults {
    tools: Vec<ToolResult>,
}

#[derive(Serialize, Debug)]
struct ToolResult {
    name: String,
    scenarios: Vec<ScenarioResult>,
    average: Option<f64>,
}

#[derive(Serialize, Debug)]
struct ScenarioResult {
    scenario: String,
    time: Option<f64>,
    error: Option<String>,
}

fn main() {
    // 1. Check dependencies
    check_binary_exists("hyperfine");

    // allow user to override binary paths via env vars
    let torrite = env::var("torrite").unwrap_or_else(|_| TORRITE_BIN.to_string());
    let mktorrent = env::var("MKTORRENT").unwrap_or_else(|_| MKTORRENT_BIN.to_string());
    let mkbrr = env::var("MKBRR").unwrap_or_else(|_| MKBRR_BIN.to_string());
    let torrenttools = env::var("TORRENTTOOLS").unwrap_or_else(|_| TORRENTTOOLS_BIN.to_string());
    let imdl = env::var("IMDL").unwrap_or_else(|_| IMDL_BIN.to_string());
    let torf = env::var("TORF").unwrap_or_else(|_| TORF_BIN.to_string());

    // Check for debug flag
    let args: Vec<String> = env::args().collect();
    let debug_mode = args.contains(&"--debug".to_string());
    let only_torrite = args.contains(&"--only-torrite".to_string());
    let json_output = args.contains(&"--json".to_string());

    check_binary_exists(&torrite);
    if !only_torrite {
        check_binary_exists(&mktorrent);
        check_binary_exists(&mkbrr);
        check_binary_exists(&torrenttools);
        check_binary_exists(&imdl);
        check_binary_exists(&torf);
    }

    if !json_output {
        println!("---------------------------------------------------");
        println!("🚀 Torrent Benchmark Runner");
        println!("   Tools being compared:");
        println!("     - torrite:      {} (V1, V2, Hybrid)", torrite);
        if !only_torrite {
            println!("     - mktorrent:    {} (V1 baseline)", mktorrent);
            println!("     - mkbrr:        {} (V1)", mkbrr);
            println!("     - imdl:         {} (V1)", imdl);
            println!("     - torf:         {} (V1)", torf);
            println!("     - torrenttools: {} (V1, V2, Hybrid)", torrenttools);
        }
        if debug_mode {
            println!("   Debug Mode: ON (Results will be kept)");
        }
        println!("---------------------------------------------------");
    }

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
    let mut tool_names = vec![
        "**torrite (V1)**",
        "**torrite (V2 Only)**",
        "**torrite (Hybrid)**",
    ];

    if !only_torrite {
        tool_names.insert(0, "mktorrent (V1)");
        tool_names.push("mkbrr (V1)");
        tool_names.push("imdl (V1)");
        tool_names.push("torf (V1)");
        tool_names.push("torrenttools (V1)");
        tool_names.push("torrenttools (V2)");
        tool_names.push("torrenttools (Hybrid)");
    }

    let mut aggregated_results: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for tool in &tool_names {
        aggregated_results.insert(tool.to_string(), Vec::new());
    }

    // 3. Run Hyperfine for each scenario
    for case in &scenarios {
        let input_path = root.join(case.path);

        if !input_path.exists() {
            if !json_output {
                println!(
                    "⚠️ Skipping '{}': Path not found {:?}",
                    case.name, input_path
                );
            }
            // Add "N/A" for missing scenarios to keep alignment
            for tool in &tool_names {
                let na_str = if tool.contains("torrite") {
                    "**N/A**".to_string()
                } else {
                    "N/A".to_string()
                };
                aggregated_results.get_mut(*tool).unwrap().push(na_str);
            }
            continue;
        }

        if !json_output {
            println!("\n▶️  Running Scenario: {}", case.name);
        }

        // Construct Output Paths (unique per scenario to allow verification)
        let safe_name = sanitize_filename(case.name);
        let out_torrite = results_dir.join(format!("{}_torrite.torrent", safe_name));
        let out_torrite_hybrid = results_dir.join(format!("{}_torrite_hybrid.torrent", safe_name));
        let out_torrite_v2 = results_dir.join(format!("{}_torrite_v2.torrent", safe_name));
        let out_mktorrent = results_dir.join(format!("{}_mktorrent.torrent", safe_name));
        let out_mkbrr = results_dir.join(format!("{}_mkbrr.torrent", safe_name));
        let out_imdl = results_dir.join(format!("{}_imdl.torrent", safe_name));
        let out_torf = results_dir.join(format!("{}_torf.torrent", safe_name));
        let out_torrenttools_v1 =
            results_dir.join(format!("{}_torrenttools_v1.torrent", safe_name));
        let out_torrenttools_v2 =
            results_dir.join(format!("{}_torrenttools_v2.torrent", safe_name));
        let out_torrenttools_hybrid =
            results_dir.join(format!("{}_torrenttools_hybrid.torrent", safe_name));

        // Format Commands
        let cmd_torrite = format_command(TORRITE_CMD, &torrite, &input_path, &out_torrite);
        let cmd_torrite_hybrid = format_command(
            TORRITE_HYBRID_CMD,
            &torrite,
            &input_path,
            &out_torrite_hybrid,
        );
        let cmd_torrite_v2 =
            format_command(TORRITE_V2_ONLY_CMD, &torrite, &input_path, &out_torrite_v2);
        let cmd_mktorrent = format_command(MKTORRENT_CMD, &mktorrent, &input_path, &out_mktorrent);
        let cmd_mkbrr = format_command(MKBRR_CMD, &mkbrr, &input_path, &out_mkbrr);
        let cmd_imdl = format_command(IMDL_CMD, &imdl, &input_path, &out_imdl);
        let cmd_torf = format_command(TORF_CMD, &torf, &input_path, &out_torf);
        let cmd_torrenttools_v1 = format_command(
            TORRENTTOOLS_V1_CMD,
            &torrenttools,
            &input_path,
            &out_torrenttools_v1,
        );
        let cmd_torrenttools_v2 = format_command(
            TORRENTTOOLS_V2_CMD,
            &torrenttools,
            &input_path,
            &out_torrenttools_v2,
        );
        let cmd_torrenttools_hybrid = format_command(
            TORRENTTOOLS_HYBRID_CMD,
            &torrenttools,
            &input_path,
            &out_torrenttools_hybrid,
        );

        let json_output_path = results_dir.join(format!("result_{}.json", safe_name));

        let cmd_prepare = format!(
            "rm -f {} {} {} {} {} {} {} {} {} {}",
            out_torrite.display(),
            out_torrite_hybrid.display(),
            out_torrite_v2.display(),
            out_mktorrent.display(),
            out_mkbrr.display(),
            out_imdl.display(),
            out_torf.display(),
            out_torrenttools_v1.display(),
            out_torrenttools_v2.display(),
            out_torrenttools_hybrid.display()
        );

        // Execute Hyperfine
        let mut hyperfine_cmd = Command::new("hyperfine");
        hyperfine_cmd
            .arg("--prepare")
            .arg(&cmd_prepare)
            .arg("--min-runs")
            .arg("3")
            .arg("--export-json")
            .arg(&json_output_path);

        // Suppress hyperfine's progress output in JSON mode (but keep stderr for errors)
        if json_output {
            hyperfine_cmd.stdout(Stdio::null());
        }

        if !only_torrite {
            hyperfine_cmd
                .arg("-n")
                .arg("mktorrent (V1)")
                .arg(&cmd_mktorrent);
        }

        hyperfine_cmd
            .arg("-n")
            .arg("**torrite (V1)**")
            .arg(&cmd_torrite)
            .arg("-n")
            .arg("**torrite (V2 Only)**")
            .arg(&cmd_torrite_v2)
            .arg("-n")
            .arg("**torrite (Hybrid)**")
            .arg(&cmd_torrite_hybrid);

        if !only_torrite {
            hyperfine_cmd.arg("-n").arg("mkbrr (V1)").arg(&cmd_mkbrr);
            hyperfine_cmd.arg("-n").arg("imdl (V1)").arg(&cmd_imdl);
            hyperfine_cmd.arg("-n").arg("torf (V1)").arg(&cmd_torf);
            hyperfine_cmd
                .arg("-n")
                .arg("torrenttools (V1)")
                .arg(&cmd_torrenttools_v1);
            hyperfine_cmd
                .arg("-n")
                .arg("torrenttools (V2)")
                .arg(&cmd_torrenttools_v2);
            hyperfine_cmd
                .arg("-n")
                .arg("torrenttools (Hybrid)")
                .arg(&cmd_torrenttools_hybrid);
        }

        let status = hyperfine_cmd.status().expect("Failed to run hyperfine");

        if !status.success() {
            eprintln!("❌ Hyperfine reported an error for scenario: {}", case.name);
            for tool in &tool_names {
                let err_str = if tool.contains("torrite") {
                    "**Err**".to_string()
                } else {
                    "Err".to_string()
                };
                aggregated_results.get_mut(*tool).unwrap().push(err_str);
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
                    let time_str = if tool.contains("torrite") {
                        format!("**{:.3}s**", mean)
                    } else {
                        format!("{:.3}s", mean)
                    };
                    aggregated_results.get_mut(*tool).unwrap().push(time_str);
                } else {
                    let missing_str = if tool.contains("torrite") {
                        "**Missing**".to_string()
                    } else {
                        "Missing".to_string()
                    };
                    aggregated_results.get_mut(*tool).unwrap().push(missing_str);
                }
            }
        }
    }

    // --- Calculate Averages and Sort ---
    let mut tool_averages: Vec<(String, f64, String)> = Vec::new();

    for tool in &tool_names {
        if let Some(times) = aggregated_results.get(*tool) {
            let mut valid_times: Vec<f64> = Vec::new();

            for time_str in times {
                // Parse time from strings like "0.123s" or "**0.456s**"
                let cleaned = time_str
                    .replace("*", "")
                    .replace("s", "")
                    .trim()
                    .to_string();
                if let Ok(time) = cleaned.parse::<f64>() {
                    valid_times.push(time);
                }
            }

            let avg = if valid_times.is_empty() {
                f64::MAX // Put entries with no valid times at the end
            } else {
                valid_times.iter().sum::<f64>() / valid_times.len() as f64
            };

            let avg_str = if tool.contains("torrite") {
                if avg == f64::MAX {
                    "**N/A**".to_string()
                } else {
                    format!("**{:.3}s**", avg)
                }
            } else {
                if avg == f64::MAX {
                    "N/A".to_string()
                } else {
                    format!("{:.3}s", avg)
                }
            };

            tool_averages.push((tool.to_string(), avg, avg_str));
        }
    }

    // Sort by average time (ascending)
    tool_averages.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    if json_output {
        // --- Generate JSON Output ---
        let mut json_results = BenchmarkResults {
            tools: Vec::new(),
        };

        for (tool, avg, _avg_str) in &tool_averages {
            let mut scenario_results = Vec::new();

            if let Some(times) = aggregated_results.get(tool.as_str()) {
                for (i, time_str) in times.iter().enumerate() {
                    let scenario_name = scenarios[i].name;

                    // Parse time from strings like "0.123s" or "**0.456s**"
                    let cleaned = time_str
                        .replace("*", "")
                        .replace("s", "")
                        .trim()
                        .to_string();

                    let (time, error) = if let Ok(time) = cleaned.parse::<f64>() {
                        (Some(time), None)
                    } else if time_str.contains("N/A") {
                        (None, Some("N/A".to_string()))
                    } else if time_str.contains("Err") {
                        (None, Some("Error".to_string()))
                    } else if time_str.contains("Missing") {
                        (None, Some("Missing".to_string()))
                    } else {
                        (None, Some(time_str.clone()))
                    };

                    scenario_results.push(ScenarioResult {
                        scenario: scenario_name.to_string(),
                        time,
                        error,
                    });
                }
            }

            let average = if *avg == f64::MAX {
                None
            } else {
                Some(*avg)
            };

            // Remove markdown bold markers from tool name
            let clean_tool_name = tool.replace("*", "");

            json_results.tools.push(ToolResult {
                name: clean_tool_name,
                scenarios: scenario_results,
                average,
            });
        }

        let json_string = serde_json::to_string_pretty(&json_results)
            .expect("Failed to serialize benchmark results to JSON");
        println!("{}", json_string);
    } else {
        // --- Generate Markdown Table ---
        println!("\n📊 Benchmark Summary\n");

        // Header
        print!("| Tool |");
        for case in &scenarios {
            print!(" {} |", case.name);
        }
        println!(" Average |");

        // Separator
        print!("|---|");
        for _ in &scenarios {
            print!("---|");
        }
        println!("---|");

        // Rows (sorted by average)
        for (tool, _avg, avg_str) in &tool_averages {
            print!("| {} |", tool);
            if let Some(times) = aggregated_results.get(tool.as_str()) {
                for time in times {
                    print!(" {} |", time);
                }
            }
            print!(" {} |", avg_str);
            println!();
        }
        println!();
    }

    // Cleanup
    if !debug_mode {
        if let Err(e) = fs::remove_dir_all(results_dir) {
            eprintln!("⚠️ Warning: Failed to clean up results directory: {}", e);
        } else if !json_output {
            println!("🧹 Cleaned up benchmark results.");
        }
    } else if !json_output {
        println!("📝 Results kept in '{}'", results_dir.display());
    }

    if !json_output {
        println!("\n✅ Benchmarks Complete.");
    }
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
