fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(tekmerion::tools::attempt_lifecycle::run(&args));
}
