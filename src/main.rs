use std::fs::File;
use std::path::Path;
use std::error::Error;

mod core;

fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("example.yml");
    let disp = path.display();
    let f = match File::open(&path) {
        Err(w) => panic!("couldn't open {}: {}", disp, w),
        Ok(f) => f,
    };

    let cfg: core::config::Config = serde_yaml::from_reader(f)?;

    println!("Regenerated:\n{}\n\n", serde_yaml::to_string(&cfg)?);

    if let Err(err) = core::config::validate(&cfg) {
        panic!("Configuration error: {:?}", err);
    }

    println!("Ok!");

    Ok(())
}
