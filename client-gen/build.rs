use std::fs;
use std::path::Path;

fn main() {
    let dest_path = Path::new("src/client.rs");
    let src_lib_path = Path::new("../client/src/lib.rs");
    let contents =
        fs::read_to_string(src_lib_path).expect("Should have been able to read the file");

    fs::write(
        &dest_path,
        format!(
            "#[macro_export]
macro_rules! generate_client {{
    () => {{
        mod executor_client {{
{}
        }}
    }}
}}",
            contents
        ),
    )
    .unwrap();
}
