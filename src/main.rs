fn main() {
    let cli = match omv::cli::parse_from_env() {
        Ok(cli) => cli,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    };

    let locale = cli
        .locale_override
        .clone()
        .unwrap_or_else(|| String::from("en-US"));
    match omv::app::run(cli) {
        Ok(output) => {
            println!("{}", output.message);
        }
        Err(err) => {
            eprintln!("{}", omv::app::render_error(&locale, &err));
            std::process::exit(1);
        }
    }
}
