fn main() {
    cc::Build::new()
        .file("src/smc_rw.c")
        .compile("smc_rw");
}
