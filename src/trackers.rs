//! Tracker-specific configurations and defaults.

/// Configuration for a specific tracker or group of trackers.
pub struct TrackerConfig {
    /// List of tracker URLs (partial matches) that share this config.
    pub urls: &'static [&'static str],
    /// Default source string to use for this tracker.
    pub default_source: Option<&'static str>,
    /// Custom piece size ranges for specific content sizes.
    pub piece_size_ranges: &'static [PieceSizeRange],
    /// Maximum piece length exponent (2^n). Default is usually 24 (16 MiB).
    pub max_piece_length: Option<u32>,
    /// Maximum .torrent file size in bytes.
    pub max_torrent_size: Option<u64>,
    /// Whether to use default piece size ranges when content size is outside custom ranges.
    pub use_default_ranges: bool,
}

/// Defines a range of content sizes and their corresponding piece size exponent.
pub struct PieceSizeRange {
    /// Maximum content size in bytes for this range.
    pub max_size: u64,
    /// Piece size exponent (2^n).
    pub piece_exp: u32,
}

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

/// Known tracker configurations.
pub static TRACKER_CONFIGS: &[TrackerConfig] = &[
    TrackerConfig {
        urls: &["anthelion.me"],
        default_source: Some("ANT"),
        piece_size_ranges: &[],
        max_piece_length: None,
        max_torrent_size: Some(250 * KIB),
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["nebulance.io"],
        default_source: Some("NBL"),
        piece_size_ranges: &[],
        max_piece_length: None,
        max_torrent_size: Some(1024 * KIB),
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["hdbits.org", "superbits.org", "sptracker.cc"],
        default_source: None,
        piece_size_ranges: &[],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: true,
    },
    TrackerConfig {
        urls: &["beyond-hd.me"],
        default_source: Some("BHD"),
        piece_size_ranges: &[],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: true,
    },
    TrackerConfig {
        urls: &["passthepopcorn.me"],
        default_source: Some("PTP"),
        piece_size_ranges: &[
            PieceSizeRange { max_size: 58 * MIB, piece_exp: 16 },    // 64 KiB
            PieceSizeRange { max_size: 122 * MIB, piece_exp: 17 },   // 128 KiB
            PieceSizeRange { max_size: 213 * MIB, piece_exp: 18 },   // 256 KiB
            PieceSizeRange { max_size: 444 * MIB, piece_exp: 19 },   // 512 KiB
            PieceSizeRange { max_size: 922 * MIB, piece_exp: 20 },   // 1 MiB
            PieceSizeRange { max_size: 3977 * MIB, piece_exp: 21 },  // 2 MiB
            PieceSizeRange { max_size: 6861 * MIB, piece_exp: 22 },  // 4 MiB
            PieceSizeRange { max_size: 14234 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 24 },    // 16 MiB
        ],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["morethantv.me"],
        default_source: Some("MTV"),
        piece_size_ranges: &[],
        max_piece_length: Some(23),
        max_torrent_size: None,
        use_default_ranges: true,
    },
    TrackerConfig {
        urls: &["empornium.sx"],
        default_source: Some("Emp"),
        piece_size_ranges: &[],
        max_piece_length: Some(23),
        max_torrent_size: None,
        use_default_ranges: true,
    },
    TrackerConfig {
        urls: &["gazellegames.net"],
        default_source: Some("GGn"),
        piece_size_ranges: &[
            PieceSizeRange { max_size: 64 * MIB, piece_exp: 15 },    // 32 KiB
            PieceSizeRange { max_size: 128 * MIB, piece_exp: 16 },   // 64 KiB
            PieceSizeRange { max_size: 256 * MIB, piece_exp: 17 },   // 128 KiB
            PieceSizeRange { max_size: 512 * MIB, piece_exp: 18 },   // 256 KiB
            PieceSizeRange { max_size: 1024 * MIB, piece_exp: 19 },  // 512 KiB
            PieceSizeRange { max_size: 2048 * MIB, piece_exp: 20 },  // 1 MiB
            PieceSizeRange { max_size: 4096 * MIB, piece_exp: 21 },  // 2 MiB
            PieceSizeRange { max_size: 8192 * MIB, piece_exp: 22 },  // 4 MiB
            PieceSizeRange { max_size: 16384 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: 32768 * MIB, piece_exp: 24 }, // 16 MiB
            PieceSizeRange { max_size: 65536 * MIB, piece_exp: 25 }, // 32 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 26 },    // 64 MiB
        ],
        max_piece_length: Some(26),
        max_torrent_size: Some(1 * MIB),
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["tracker.alpharatio.cc"],
        default_source: Some("AlphaRatio"),
        piece_size_ranges: &[
            PieceSizeRange { max_size: 64 * MIB, piece_exp: 15 },    // 32 KiB
            PieceSizeRange { max_size: 128 * MIB, piece_exp: 16 },   // 64 KiB
            PieceSizeRange { max_size: 256 * MIB, piece_exp: 17 },   // 128 KiB
            PieceSizeRange { max_size: 512 * MIB, piece_exp: 18 },   // 256 KiB
            PieceSizeRange { max_size: 1024 * MIB, piece_exp: 19 },  // 512 KiB
            PieceSizeRange { max_size: 2048 * MIB, piece_exp: 20 },  // 1 MiB
            PieceSizeRange { max_size: 4096 * MIB, piece_exp: 21 },  // 2 MiB
            PieceSizeRange { max_size: 8192 * MIB, piece_exp: 22 },  // 4 MiB
            PieceSizeRange { max_size: 16384 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: 32768 * MIB, piece_exp: 24 }, // 16 MiB
            PieceSizeRange { max_size: 65536 * MIB, piece_exp: 25 }, // 32 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 26 },    // 64 MiB
        ],
        max_piece_length: Some(26),
        max_torrent_size: Some(2 * MIB),
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["seedpool.org"],
        default_source: Some("seedpool.org"),
        piece_size_ranges: &[
            PieceSizeRange { max_size: 64 * MIB, piece_exp: 15 },     // 32 KiB
            PieceSizeRange { max_size: 128 * MIB, piece_exp: 16 },    // 64 KiB
            PieceSizeRange { max_size: 256 * MIB, piece_exp: 17 },    // 128 KiB
            PieceSizeRange { max_size: 512 * MIB, piece_exp: 18 },    // 256 KiB
            PieceSizeRange { max_size: 1024 * MIB, piece_exp: 19 },   // 512 KiB
            PieceSizeRange { max_size: 2048 * MIB, piece_exp: 20 },   // 1 MiB
            PieceSizeRange { max_size: 4096 * MIB, piece_exp: 21 },   // 2 MiB
            PieceSizeRange { max_size: 8192 * MIB, piece_exp: 22 },   // 4 MiB
            PieceSizeRange { max_size: 16384 * MIB, piece_exp: 23 },  // 8 MiB
            PieceSizeRange { max_size: 32768 * MIB, piece_exp: 24 },  // 16 MiB
            PieceSizeRange { max_size: 65536 * MIB, piece_exp: 25 },  // 32 MiB
            PieceSizeRange { max_size: 131072 * MIB, piece_exp: 26 }, // 64 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 27 },     // 128 MiB
        ],
        max_piece_length: Some(27),
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["norbits.net"],
        default_source: None,
        piece_size_ranges: &[
            PieceSizeRange { max_size: 250 * MIB, piece_exp: 18 },   // 256 KiB
            PieceSizeRange { max_size: 1024 * MIB, piece_exp: 20 },  // 1 MiB
            PieceSizeRange { max_size: 5120 * MIB, piece_exp: 21 },  // 2 MiB
            PieceSizeRange { max_size: 20480 * MIB, piece_exp: 22 }, // 4 MiB
            PieceSizeRange { max_size: 40960 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 24 },    // 16 MiB
        ],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["landof.tv"],
        default_source: None,
        piece_size_ranges: &[
            PieceSizeRange { max_size: 32 * MIB, piece_exp: 15 },   // 32 KiB
            PieceSizeRange { max_size: 62 * MIB, piece_exp: 16 },   // 64 KiB
            PieceSizeRange { max_size: 125 * MIB, piece_exp: 17 },  // 128 KiB
            PieceSizeRange { max_size: 250 * MIB, piece_exp: 18 },  // 256 KiB
            PieceSizeRange { max_size: 500 * MIB, piece_exp: 19 },  // 512 KiB
            PieceSizeRange { max_size: 1000 * MIB, piece_exp: 20 }, // 1 MiB
            PieceSizeRange { max_size: 1945 * MIB, piece_exp: 21 }, // 2 MiB
            PieceSizeRange { max_size: 3906 * MIB, piece_exp: 22 }, // 4 MiB
            PieceSizeRange { max_size: 7810 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 24 },   // 16 MiB
        ],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["torrent-syndikat.org", "tee-stube.org"],
        default_source: None,
        piece_size_ranges: &[
            PieceSizeRange { max_size: 250 * MIB, piece_exp: 20 },   // 1 MiB
            PieceSizeRange { max_size: 1024 * MIB, piece_exp: 20 },  // 1 MiB
            PieceSizeRange { max_size: 5120 * MIB, piece_exp: 20 },  // 1 MiB
            PieceSizeRange { max_size: 20480 * MIB, piece_exp: 22 }, // 4 MiB
            PieceSizeRange { max_size: 51200 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 24 },    // 16 MiB
        ],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["lst.gg"],
        default_source: Some("lst.gg"),
        piece_size_ranges: &[
            PieceSizeRange { max_size: 1024 * MIB, piece_exp: 20 },  // 1 MiB
            PieceSizeRange { max_size: 4096 * MIB, piece_exp: 21 },  // 2 MiB
            PieceSizeRange { max_size: 12288 * MIB, piece_exp: 22 }, // 4 MiB
            PieceSizeRange { max_size: 20480 * MIB, piece_exp: 23 }, // 8 MiB
            PieceSizeRange { max_size: u64::MAX, piece_exp: 24 },    // 16 MiB
        ],
        max_piece_length: Some(24),
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["aither.cc"],
        default_source: Some("Aither"),
        piece_size_ranges: &[],
        max_piece_length: None,
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["upload.cx"],
        default_source: Some("ULCX"),
        piece_size_ranges: &[],
        max_piece_length: None,
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["capybarabr.com"],
        default_source: Some("CapybaraBR"),
        piece_size_ranges: &[],
        max_piece_length: None,
        max_torrent_size: None,
        use_default_ranges: false,
    },
    TrackerConfig {
        urls: &["hawke.uno"],
        default_source: Some("HUNO"),
        piece_size_ranges: &[],
        max_piece_length: None,
        max_torrent_size: None,
        use_default_ranges: false,
    },
];

/// Returns the config for a given tracker URL.
pub fn find_tracker_config(tracker_url: &str) -> Option<&'static TrackerConfig> {
    for config in TRACKER_CONFIGS {
        for url in config.urls {
            if tracker_url.contains(url) {
                return Some(config);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_tracker_config() {
        // Known trackers
        assert!(find_tracker_config("https://passthepopcorn.me/announce").is_some());
        assert!(find_tracker_config("http://gazellegames.net/announce.php").is_some());
        assert!(find_tracker_config("https://anthelion.me/announce").is_some());
        
        // Check specific values for PTP
        let ptp = find_tracker_config("passthepopcorn.me").unwrap();
        assert_eq!(ptp.default_source, Some("PTP"));
        assert!(!ptp.use_default_ranges);
        
        // Check specific values for GGn
        let ggn = find_tracker_config("gazellegames.net").unwrap();
        assert_eq!(ggn.default_source, Some("GGn"));
        
        // Unknown tracker
        assert!(find_tracker_config("https://example.com/announce").is_none());
    }

    #[test]
    fn test_tracker_defaults_integrity() {
        for config in TRACKER_CONFIGS {
            // Ensure every config has at least one URL
            assert!(!config.urls.is_empty());
            
            // Check range consistency if present
            if !config.piece_size_ranges.is_empty() {
                let mut last_max = 0;
                for range in config.piece_size_ranges {
                    assert!(range.max_size > last_max);
                    last_max = range.max_size;
                }
            }
        }
    }
}
