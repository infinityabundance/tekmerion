fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(tekmerion::tools::run_traces_inject::run(&args));
}
