# querypath-snapshot

[![Crates.io](https://img.shields.io/crates/v/querypath-snapshot.svg)](https://crates.io/crates/querypath-snapshot)
[![Docs.rs](https://docs.rs/querypath-snapshot/badge.svg)](https://docs.rs/querypath-snapshot)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Unlicense-blue.svg)](LICENSE)

An automated Markdown snapshot and code documentation generator designed for the **querypath** ecosystem.

Part of the querypath family:
- [![Crates.io](https://img.shields.io/crates/v/querypath.svg)](https://crates.io/crates/querypath) **querypath** – Fast filesystem scanner and pattern engine.
- [![Crates.io](https://img.shields.io/crates/v/querypath-fmt.svg)](https://crates.io/crates/querypath-fmt) **querypath-fmt** – Advanced tree layout and sorting formatter.
- [![Crates.io](https://img.shields.io/crates/v/temporal-fmt.svg)](https://crates.io/crates/temporal-fmt) **temporal-fmt** – Standalone timestamp formatting engine.

## Features

- **Automated Snapshot Generation:** Converts `querypath::QueryResults` and `querypath_fmt::QueryPathFmt` into structured Markdown files.
- **Timestamp Versioning:** Formats dynamic execution tokens (using `temporal-fmt`).
- **Smart File Embedding:** Includes numbered text file blocks, automatically skipping binaries and oversized files exceeding `max_single_file_size`.
- **Double Structure Verification:** Encloses the source code sections with header and footer directory tree representations.

## Example

```rust
use anyhow::Result;
use querypath::QueryPath;
use querypath_fmt::QueryPathFmt;
use querypath_snapshot::Snapshot;

fn main() -> Result<()> {
    let res = QueryPath::new()
        .scan_at(["./"])
        .match_pattern(["!**/{.git|target}/?**"])
        .run()?;

    let fmt = QueryPathFmt::new();

    let output_path = Snapshot::new()
        .title("MY CODE SNAPSHOT")
        .output_dir("./prompt")
        .max_single_file_size(256 * 1024) // Maximum allowed size per individual source file (256 KiB)
        .generate_and_save(&res, &fmt)?;

    println!("Snapshot saved to: {}", output_path.display());
    Ok(())
}
```

---
