macro_rules! doc {
    () => {
        match &crate::args::ARGS.command {
            Some(crate::args::Commands::Doc(x)) => Some(x),
            _ => None
        }.unwrap()
    }
}

pub mod walk;
pub mod build;
pub mod reader;
