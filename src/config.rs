use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize, Deserializer};
use std::fmt;
use std::path::PathBuf;
use std::marker::PhantomData;
use std::str::FromStr;
use serde::de::{self, Visitor, MapAccess};
use void::Void;


/*
templates:
  web:
    module: http
    https: compatible # enforcing (yes), only, disabled (no)
    port:
      http: 80
      https: 443
*/

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ConfigHttpHttps {
    Only,
    #[serde(rename = "hsts", rename_all = "camelCase")]
    HSTS {
        duration: u64,
        #[serde(default)]
        include_sub_domains: bool,
        #[serde(default)]
        preload: bool,
    },
    #[serde(alias = "override", alias = "enforcing", alias = "yes")]
    Enforcing,
	Compatible,
	#[serde(alias = "no")]
	Disabled,
}

fn http_default_port() -> u16 { 80 }
fn https_default_port() -> u16 { 80 }

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigHttpPort {
	#[serde(default = "http_default_port")]
	pub http: u16,
	#[serde(default = "https_default_port")]
	pub https: u16,
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "module")]
pub enum ConfigServerTemplate {
	Http {
		https: ConfigHttpHttps,
		port: ConfigHttpPort,
	},
}


/*
servers:
  - template: web
    host: tespent.cn
    location: /git

    backend:
      type: proxy
      target: 127.20.1.1:32
*/

fn rewrite_default_code() -> u16 { 302 }

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ConfigBackend {
	Proxy {
		target: String,
	},
	Rewrite {
		target: String,
		#[serde(default = "rewrite_default_code")]
		code: u16,
	},
	File {
		path: PathBuf,
	},
}

impl FromStr for ConfigBackend {
	type Err = Void;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(ConfigBackend::File {
			path: PathBuf::from(s)
		})
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigServer {
	pub name: Option<String>,

	pub template: String,
	pub host: String,
	pub location: Option<String>,

	#[serde(deserialize_with = "string_or_struct")]
	pub backend: ConfigBackend,
}





#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
	#[serde(default)]
	pub servers: Vec<ConfigServer>,

	#[serde(default)]
	pub templates: Map<String, ConfigServerTemplate>,
}



fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

pub trait TemplateWriter {
    fn to_config_string(&self) -> String;
}

pub struct WebTemplate<'a> {
    host: &'a String,
	servers: Vec<&'a ConfigServer>,
    template: &'a ConfigServerTemplate,
}

fn escape_nginx_string(s: &str) -> String {
    let mut retval = String::new();
    let mut use_quotes = false;
    for ch in s.bytes() {
        let escaped = std::ascii::escape_default(ch);
        if escaped.len() != 1 || ch == b' ' || ch == b';' {
            use_quotes = true;
        }
        retval.extend(escaped.map(|c| c as char));
    };

    if use_quotes {
        "\"".to_owned() + &retval + "\""
    } else {
        retval
    }
}

/*

# server
- host:
    - tespent.cn
  listen:
    - port: 80
      attr: []
    - port: 443
      attr:
        - ssl
        - http2
  global:
    - index index.html;
  subserver:
    /:
      - someserver
    /sub:
      - someserver
  server:
    - someserver


*/

impl TemplateWriter for WebTemplate<'_> {
    fn to_config_string(&self) -> String {
        let mut retv: String = String::new();

        match &self.template {
            ConfigServerTemplate::Http { https, port } => {
                let ident_config = format!("  server_name {};\n", self.host);
                let mut server_str = format!("  # found {} backend(s) for host {}\n", self.servers.len(), self.host);

                for server in &self.servers {
                    let block_name = match &server.name.as_ref() {
                        Some(s) => s.to_owned(),
                        None => "<anonymous>",
                    };

                    let backend_str = match &server.backend {
                        ConfigBackend::Proxy { target } => {
                            format!("    proxy_pass {};\n", escape_nginx_string(target))
                        },
                        ConfigBackend::Rewrite { target, code } => {
                            format!("    rewrite {} {};\n", escape_nginx_string(target), code)
                        },
                        ConfigBackend::File { path } => {
                            format!("    root {};\n", escape_nginx_string(&path.to_string_lossy()))
                        },
                    };

                    server_str += &format!("  # generated block for config {}\n", block_name);
                    if let Some(location) = &server.location {
                        server_str += &format!("  location {} {{\n", escape_nginx_string(location));
                        server_str += &backend_str;
                        server_str += "  }\n";
                    } else {
                        server_str += "  # default location\n  #{\n";
                        server_str += &backend_str;
                        server_str += "  #}\n";
                    }
                }

                match https {
                    ConfigHttpHttps::Enforcing => {
                        retv += "server {\n";
                        retv += &format!("  listen {} ssl http2;\n", port.https);
                        retv += &ident_config;
                        retv += &server_str;
                        retv += "}\n";
                        retv += "server {\n";
                        retv += &format!("  listen {};\n", port.http);
                        retv += &ident_config;
                        retv += "  rewrite . https://$host$request_uri permanent;\n";
                        retv += "}\n";
                    }
                    ConfigHttpHttps::HSTS { duration, include_sub_domains, preload } => {
                        retv += "server {\n";
                        retv += &format!("  listen {} ssl http2;\n", port.https);
                        retv += &ident_config;
                        {
                            retv += &format!("  add_header Strict-Transport-Security \"max-age={}", duration);
                            if *include_sub_domains {
                                retv += "; includeSubDomains";
                            }
                            if *preload {
                                retv += "; preload";
                            }
                            retv += "\"\n";
                        }
                        retv += &server_str;
                        retv += "}\n";
                        retv += "server {\n";
                        retv += &format!("  listen {};\n", port.http);
                        retv += &ident_config;
                        retv += "  rewrite . https://$host$request_uri permanent;\n";
                        retv += "}\n";
                    },
                    ConfigHttpHttps::Compatible => {
                        retv += "server {\n";
                        retv += &format!("  listen {};\n", port.http);
                        retv += &format!("  listen {} ssl http2;\n", port.https);
                        retv += &ident_config;
                        retv += &server_str;
                        retv += "}\n";
                    },
                    ConfigHttpHttps::Disabled => {
                        retv += "server {\n";
                        retv += &format!("  listen {};\n", port.http);
                        retv += &ident_config;
                        retv += &server_str;
                        retv += "}\n";
                    },
                    ConfigHttpHttps::Only => {
                        retv += "server {\n";
                        retv += &format!("  listen {} ssl http2;\n", port.https);
                        retv += &ident_config;
                        retv += &server_str;
                        retv += "}\n";
                    },
                }
            }
        };

        retv
    }
}

pub fn validate(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {

    let mut web: Map<String, WebTemplate> = Map::new();

    for r in &cfg.servers {
        if !cfg.templates.contains_key(&r.template) {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unknown template {}", r.template))))
        }

        let server_block = web.entry(r.host.to_owned()).or_insert(WebTemplate {
            host: &r.host,
            servers: vec![],
            template: cfg.templates.get(&r.template).unwrap(),
        });

        server_block.servers.push(r);
    }
    for (_, tmpl) in web {
        println!("{}", tmpl.to_config_string());
    }

    Ok(())
}
