use std::path::Path;

use extricrate::dependencies::UseStatements;

pub fn transform(input_path: &Path, output_path: &Path, use_statements: UseStatements) {
    println!("{:?}", input_path);
    println!("{:?}", output_path);
    println!("{:?}", use_statements);
}
