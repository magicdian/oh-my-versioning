fn main() {
    let raw_args = std::env::args().skip(1).collect::<Vec<_>>();
    let output_mode = omv::cli::detect_output_mode(&raw_args);
    let locale =
        omv::cli::detect_locale_override(&raw_args).unwrap_or_else(|| String::from("en-US"));

    let cli = match omv::cli::parse_args(raw_args) {
        Ok(cli) => cli,
        Err(err) => {
            match output_mode {
                omv::cli::OutputMode::Json => {
                    eprintln!("{}", omv::app::render_structured_error("cli", &err));
                }
                omv::cli::OutputMode::Text => {
                    eprintln!("{}", omv::app::render_error(&locale, &err));
                }
            }
            std::process::exit(2);
        }
    };

    match omv::app::run(cli) {
        Ok(output) => {
            println!("{}", output.message);
        }
        Err(err) => {
            match output_mode {
                omv::cli::OutputMode::Json => {
                    eprintln!("{}", omv::app::render_structured_error("runtime", &err));
                }
                omv::cli::OutputMode::Text => {
                    eprintln!("{}", omv::app::render_error(&locale, &err));
                }
            }
            std::process::exit(1);
        }
    }
}
