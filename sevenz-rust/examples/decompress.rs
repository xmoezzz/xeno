use std::time::Instant;

use sevenz_rust::default_entry_extract_fn;

fn main() {
    let instant = Instant::now();
    // sevenz_rust::decompress_file("examples/data/sample.7z", "examples/data/sample").expect("complete");
    sevenz_rust::decompress_file_with_extract_fn(
        "examples/data/sample.7z",
        "examples/data/sample",
        |entry, reader, dest| {
            println!("start extract {}", entry.name());
            let r = default_entry_extract_fn(entry, reader, dest);
            println!("complete extract {}", entry.name());
            r
        },
    )
    .expect("complete");
    println!("decompress done:{:?}", instant.elapsed());
}
