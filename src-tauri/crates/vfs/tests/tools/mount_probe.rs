use std::path::PathBuf;

fn main() {
    let cpk = std::env::args()
        .nth(1)
        .expect("usage: mount_probe <cpk path>");
    let cache = PathBuf::from(std::env::var("TEMP").unwrap_or_default()).join("vw-mount-probe");
    let mount = vfs::Mount::new(Some(cache));

    println!("split        = {:?}", vfs::split(&cpk));
    println!("is_archive   = {}", vfs::is_archive_path(&cpk));

    match mount.list(&cpk) {
        Ok(listing) => {
            println!(
                "list root    = ok  dirs={} files={} hidden={}",
                listing.dirs.len(),
                listing.files.len(),
                listing.hidden_files
            );
            for dir in listing.dirs.iter().take(3) {
                println!("   dir {:?}", dir.path);
                match mount.list(&dir.path) {
                    Ok(nested) => println!(
                        "      ok dirs={} files={} hidden={}",
                        nested.dirs.len(),
                        nested.files.len(),
                        nested.hidden_files
                    ),
                    Err(error) => println!("      FAIL {error}"),
                }
            }
            if let Some(file) = listing.files.first() {
                println!("   file {:?}", file.path);
                match mount.read(&file.path) {
                    Ok(bytes) => println!("      read ok {} bytes", bytes.len()),
                    Err(error) => println!("      read FAIL {error}"),
                }
                match mount.sibling_container(&file.path) {
                    Some(container) => {
                        println!("      container {container}");
                        match mount.read(&container) {
                            Ok(bytes) => println!("      container read ok {} bytes", bytes.len()),
                            Err(error) => println!("      container read FAIL {error}"),
                        }
                    }
                    None => println!("      no container"),
                }
            }
        }
        Err(error) => println!("list root    = FAIL {error}"),
    }

    match mount.summary(&cpk) {
        Ok(summary) => println!(
            "summary      = entries={} dirs={} compressed={}",
            summary.entries, summary.directories, summary.compressed
        ),
        Err(error) => println!("summary      = FAIL {error}"),
    }
}
