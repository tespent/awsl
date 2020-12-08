use std::fs::File;
use std::path::Path;
use std::error::Error;
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ConfigBackend {
    Full {
        #[serde(rename = "type")]
        backend_type: String,
        target: String,
    },
    Simple(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ConfigScope {
    Plain(String),
    List(Vec<String>),
    Full {
        include: Option<Box<ConfigScope>>,
        exclude: Option<Box<ConfigScope>>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfigServer {
    host: String,
    backend: ConfigBackend,
    policy: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag="default",rename_all = "snake_case")]
enum ConfigPolicyScope {
    Allow {
        #[serde(rename = "deny")]
        deny_scope: Option<ConfigScope>,
    },
    Deny {
        #[serde(rename = "allow")]
        allow_scope: Option<ConfigScope>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ConfigPolicyTemplating {
    Ignore,
    Simple {
        message: String,
    },
    List {
        message: Vec<String>
    },
    Raw(BTreeMap<String, String>),
}

impl Default for ConfigPolicyTemplating {
    fn default() -> ConfigPolicyTemplating { ConfigPolicyTemplating::Ignore }
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfigPolicy {
    #[serde(flatten)]
    scope: ConfigPolicyScope,
    #[serde(default)]
    #[serde(flatten)]
    message: ConfigPolicyTemplating,
    template: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    servers: Vec<ConfigServer>,
    policy: BTreeMap<String, ConfigPolicy>,
    scopes: BTreeMap<String, ConfigScope>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("example.yaml");
    let disp = path.display();
    let f = match File::open(&path) {
        Err(w) => panic!("couldn't open {}: {}", disp, w),
        Ok(f) => f,
    };

    let cfg: Config = serde_yaml::from_reader(f)?;

    println!("{:?}\n{}", cfg, serde_yaml::to_string(&cfg)?);

    Ok(())
}
