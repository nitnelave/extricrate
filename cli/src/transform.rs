use std::{
    fs::{File, read_to_string},
    path::Path,
};

pub fn transform(input_path: &str, output_path: &str, use_statements: &str) {
    if !Path::new(output_path).exists() {
        File::create(output_path).expect("Err: failed to create a file");
    }

    let content = read_to_string(input_path).expect("Err: failed to read the file content");
    let line_with_use = content
        .lines()
        .filter(|line| line.contains(use_statements))
        .next()
        .unwrap();

    println!("{}", line_with_use.replace(line_with_use, "todo()!"));
}
