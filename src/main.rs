mod modules;

fn main() {
    match modules::cli::read_command() {
        Ok(mut app) => app.run(), 
        Err(err) => eprintln!("{}", err),
    };
}
