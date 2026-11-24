fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(tekmerion::tools::sealed_court::run(&args));
}
