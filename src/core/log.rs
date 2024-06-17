use simplelog::*;

pub fn initialize_logging(level: LevelFilter) {
    CombinedLogger::init(vec![
        TermLogger::new(
            level,
            Config::default(),
            TerminalMode::Stderr,
            ColorChoice::Auto,
        ),
        // WriteLogger::new(LevelFilter::Info, Config::default(), filestream),
    ])
    .unwrap();
}
