use std::fs::File;
use std::path::Path;
use std::error::Error;

mod config;

fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("example.yml");
    let disp = path.display();
    let f = match File::open(&path) {
        Err(w) => panic!("couldn't open {}: {}", disp, w),
        Ok(f) => f,
    };

    let cfg: config::Config = serde_yaml::from_reader(f)?;

    println!("{:?}\n{}", cfg, serde_yaml::to_string(&cfg)?);

    if let Err(err) = config::validate(&cfg) {
        panic!("Configuration error: {:?}", err);
    }

    println!("Ok!");

    Ok(())
}
