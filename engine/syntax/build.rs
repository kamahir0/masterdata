fn main() {
    cc::Build::new().std("c11").include("src").file("src/parser.c").file("src/scanner.c").warnings(false).compile("tree-sitter-yaml");
    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/scanner.c");
}
