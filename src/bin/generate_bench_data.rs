use rand::{Rng, RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

// Helper to generate a single file with random content
fn generate_file(path: &Path, size: u64, seed: u64) -> std::io::Result<()> {
    // Ensure parent dir exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut rng = XorShiftRng::seed_from_u64(seed);
    let f = File::create(path)?;
    let mut writer = BufWriter::with_capacity(65536, f); // Explicit 64KB buffer cap

    // 64KB writing buffer
    let mut buffer = [0u8; 65536];
    let mut remaining = size;

    while remaining > 0 {
        let to_write = std::cmp::min(remaining, buffer.len() as u64) as usize;
        rng.fill_bytes(&mut buffer[0..to_write]);
        writer.write_all(&buffer[0..to_write])?;
        remaining -= to_write as u64;
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let root = Path::new("benchmark_data");
    // Clean start (optional, be careful with this in prod)
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root)?;

    let mut rng = rand::thread_rng();

    // =========================================================
    // 1. The Monolith (Sequential Throughput Test)
    // =========================================================
    // One 5GB file.
    println!("[1/4] Generating huge 5GB ISO...");
    generate_file(
        &root.join("distro_images/huge_distro.iso"),
        5 * 1024 * 1024 * 1024,
        1,
    )?;

    // =========================================================
    // 2. The "Project" (IOPS & Metadata Test)
    // =========================================================
    // Simulates a large git repo (node_modules or target dir).
    // Thousands of tiny files (1KB - 20KB) in nested folders.
    println!("[2/4] Generating simulated source code (nested tiny files)...");
    let src_root = root.join("src_tree");

    // Create 50 "modules", each with varied depth
    for module_id in 0..50 {
        let mut path = src_root.join(format!("module_{}", module_id));

        // Random nesting depth 2-5 levels
        let depth = rng.gen_range(2..6);
        for d in 0..depth {
            path = path.join(format!("sub_pkg_{}", d));
        }

        // Create 20-50 files per module
        let file_count = rng.gen_range(20..50);
        for f in 0..file_count {
            let size = rng.gen_range(500..20_000); // 500 bytes to 20KB
            let ext = if f % 2 == 0 { "rs" } else { "json" };
            generate_file(
                &path.join(format!("file_{}.{}", f, ext)),
                size,
                module_id as u64 + f as u64,
            )?;
        }
    }

    // =========================================================
    // 3. The "Photo Album" (Mixed Small/Medium)
    // =========================================================
    // Simulates random documents and photos.
    // 500 files, varying from 100KB to 15MB.
    println!("[3/4] Generating documents and images...");
    let doc_root = root.join("user_documents");

    for i in 0..500 {
        // Skew distribution: mostly small (images), some larger (raw/pdf)
        let size = if rng.gen_bool(0.8) {
            rng.gen_range(100 * 1024..3 * 1024 * 1024) // 100KB - 3MB
        } else {
            rng.gen_range(5 * 1024 * 1024..15 * 1024 * 1024) // 5MB - 15MB
        };

        let ext_list = ["jpg", "png", "docx", "pdf"];
        let ext = ext_list[rng.gen_range(0..ext_list.len())];

        generate_file(&doc_root.join(format!("doc_{}.{}", i, ext)), size, i as u64)?;
    }

    // =========================================================
    // 4. The "Work Assets" (Medium-Large)
    // =========================================================
    // Simulates video assets, large binaries, object files.
    // 20 files, 50MB to 500MB each.
    println!("[4/4] Generating large assets...");
    let asset_root = root.join("assets");

    for i in 0..20 {
        let size = rng.gen_range(50 * 1024 * 1024..500 * 1024 * 1024); // 50MB - 500MB
        generate_file(
            &asset_root.join(format!("raw_footage_{}.mp4", i)),
            size,
            i as u64,
        )?;
    }

    // =========================================================
    // 5. The "Piece Boundary" Stress Test
    // =========================================================
    // Creating files around common power-of-2 boundaries (256KB, 512KB, 1MB, 4MB)
    // to verify the hasher doesn't drop bytes at boundaries.
    println!("[5/7] Generating boundary edge cases...");
    let edge_root = root.join("edge_cases");
    let piece_sizes = [256 * 1024, 512 * 1024, 1024 * 1024, 4 * 1024 * 1024]; // Common piece sizes

    for &p_size in &piece_sizes {
        let p_dir = edge_root.join(format!("piece_{}", p_size));

        // Exact match
        generate_file(&p_dir.join("exact.bin"), p_size, p_size)?;
        // Off by one byte (under)
        generate_file(&p_dir.join("minus_one.bin"), p_size - 1, p_size)?;
        // Off by one byte (over - starts new piece with 1 byte)
        generate_file(&p_dir.join("plus_one.bin"), p_size + 1, p_size)?;
        // Prime number size (guaranteed misalignment)
        generate_file(&p_dir.join("prime_misalign.bin"), p_size + 17, p_size)?;
    }

    // =========================================================
    // 6. The "Metadata Bomb" (Many Empty/Tiny Files)
    // =========================================================
    // 10,000 files of 0 bytes or 1 byte.
    // This stresses the logic that builds the .torrent dictionary structure.
    println!("[7/7] Generating metadata swarm...");
    let swarm_root = root.join("swarm_stress");
    fs::create_dir_all(&swarm_root)?;

    for i in 0..10_000 {
        // Just create empty files or 1 byte files
        let path = swarm_root.join(format!("tiny_{}.bin", i));
        // We use std::fs directly for speed here to avoid our rng overhead for 0 bytes
        if i % 2 == 0 {
            File::create(path)?; // 0 bytes
        } else {
            // 1 byte
            let mut f = File::create(path)?;
            f.write_all(&[1u8])?;
        }
    }

    println!("Done. Benchmark data set created in ./benchmark_data");
    Ok(())
}
