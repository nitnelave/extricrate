use std::{fs, path::Path};

pub fn transform(module: &str, destination_crate: &str) {
    let module_path = format!("src/{}", module);
    let pathway = fs::read_dir(module_path).expect("Err: failed to read the directory");

    // Combining the pathway
    let mut combined_path: Vec<String> = Vec::new();
    for path in pathway {
        let path_way = path.unwrap().path().display().to_string();
        combined_path.push(path_way);
    }

    // Checking and creating the directory
    if !Path::new(destination_crate).exists() {
        fs::create_dir_all(destination_crate).expect("Err: failed to create a directory");
    }

    // Taking the file from the path
    for data in combined_path.into_iter() {
        let split_data: Vec<&str> = data.split('/').collect();
        let destination_path = format!("{}/{}", destination_crate, split_data[2]);
        fs::copy(data, destination_path).expect("Err: failed to copy the files");
    }
}
