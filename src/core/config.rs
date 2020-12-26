use std::collections::BTreeMap as Map;
use serde::{Serialize, Deserialize, Deserializer};
use std::fmt;
use std::path::PathBuf;
use std::marker::PhantomData;
use std::str::FromStr;
use serde::de::{self, Visitor, MapAccess, SeqAccess};
type Void = std::convert::Infallible;

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
	#[serde(deserialize_with = "string_or_list")]
	pub host: Vec<String>,
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

fn string_or_list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrList<T>(PhantomData<fn() -> Vec<T>>);

    impl<'de, T> Visitor<'de> for StringOrList<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {   
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or sequence<string>")
        }

        fn visit_str<E>(self, value: &str) -> Result<Vec<T>, E>
        where
            E: de::Error,
        {
            Ok(vec![FromStr::from_str(value).unwrap()])
        }

        fn visit_seq<S>(self, seq: S) -> Result<Vec<T>, S::Error>
        where
            S: SeqAccess<'de>,
        {
            // `SeqAccessDeserializer` is a wrapper that turns a `SeqAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the sequence visitor.
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(StringOrList(PhantomData))
}

pub fn validate(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
