mod module_a {
    use crate::module_a::module_b;
    mod module_b {
        use foo::Bar;
    }
}

fn main() {}
