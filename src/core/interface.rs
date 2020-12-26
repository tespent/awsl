use std::collections::BTreeMap;
use std::sync::Arc;

/*

id1:
  host
    - some.site
  listen:
    - port: 80
      attr: []
    - port: 443
      attr:
        - ssl
        - http2
  global:
    - blablabla
  subservers:
    /path1:
      - blablabla
    /path2:
      - blablabla
  server:
    blablabla

*/

pub use std::error::Error as Error;

pub enum OverwritePolicy {
    Error,
    Ignore,
    Overwrite,
}

pub trait BackendDescriptor: std::fmt::Debug {
    fn get_key(&self) -> String; // should be unique
    fn to_backend_config(&self) -> Result<String, Box<dyn Error>>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ServerInterfaceAttribute {
    Http, Https
}

#[derive(Copy, Clone, PartialEq)]
pub struct ServerInterface {
    port: u16,
    attr: ServerInterfaceAttribute,
}

impl std::fmt::Debug for ServerInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}:{}", self.attr, self.port))
    }
}

#[derive(Clone)]
pub struct WebServerInstance {
    host: Vec<String>,
    interface: Vec<ServerInterface>,
    location: Option<String>,
    descriptor: Arc<dyn BackendDescriptor>,
}

#[derive(Clone)]
pub struct WebServer {
    host: Vec<String>,
    interface: Vec<ServerInterface>,

    subservers: BTreeMap<String, Arc<dyn BackendDescriptor>>,
    server: Option<Arc<dyn BackendDescriptor>>,
}

impl std::fmt::Debug for WebServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("WebServer {{host={:?}, interface={:?}}}", self.host, self.interface))
    }
}


pub trait WebRegistry {
    fn add_server(&mut self, inst: &WebServerInstance, policy: OverwritePolicy) -> Result<&mut Self, Box<dyn Error>>;
    fn clear(&mut self);

    fn get_web_servers(&self) -> &Vec<WebServer>;
}

pub struct Registry {
    web: Vec<WebServer>,
}

impl std::default::Default for Registry {
    fn default() -> Self {
        Registry {
            web: Vec::new(),
        }
    }
}

// impl Registry {
//     fn key_from_server_address(host: &Vec<String>, interface: &Vec<ServerInterface>) -> String {
//         host.join(",") + "-" + &interface.iter().map(|x| format!("{:?}:{}", x.attr, x.port)).collect::<String>()
//     }
// }

#[cfg(test)]
macro_rules! test_println {
    ($l: literal) => {
        eprintln!(concat!("Registry: ", $l));
    };
    ($l: literal, $($e: expr),+) => {
        eprintln!(concat!("Registry: ", $l), $($e),+);
    };
}

#[cfg(not(test))]
macro_rules! test_println {
    ($l: literal) => {
        ()
    };
    ($l: literal, $($e: expr),+) => {
        ()
    };
}

macro_rules! execute_overwrite_policy {
    ($policy: expr, $check: expr, $w: stmt) => {
        execute_overwrite_policy!($policy, $check, $w,
            Box::new(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Cannot override existed value"))
        )
    };
    ($policy: expr, $check: expr, $w: stmt, $err: literal) => {
        execute_overwrite_policy!($policy, $check, $w,
            Box::new(std::io::Error::new(std::io::ErrorKind::AlreadyExists, $err))
        )
    };

    ($policy: expr, $check: expr, $w: stmt, $err: expr) => {
        if $check {
            match $policy {
                OverwritePolicy::Error => {
                    return Err($err);
                },
                OverwritePolicy::Ignore => {
                    // do nothing
                },
                OverwritePolicy::Overwrite => {
                    $w
                },
            }
        } else {
            $w
        }
    };
}

impl WebRegistry for Registry {
    fn add_server(&mut self, inst: &WebServerInstance, policy: OverwritePolicy) -> Result<&mut Self, Box<dyn Error>> {
        test_println!("Add server");

        if inst.host.len() == 0 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "host is empty list")));
        }
        if inst.interface.len() == 0 {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "interface is empty list")));
        }
        let mut pairs = Vec::new();
        pairs.push((inst.host.clone(), inst.interface.clone()));

        while pairs.len() > 0 {
            let (hosts, interfaces_for_all_hosts) = &mut pairs[0];
            let mut new_pairs = Vec::new();

            test_println!("Processing pair ({:?}, {:?})", hosts, interfaces_for_all_hosts);

            let mut i = 0;
            while i < hosts.len() {
                let host = hosts[i].clone();
                let mut interfaces = interfaces_for_all_hosts.clone();

                let mut new_hosts = Vec::new();
                'host_search_loop: for web_host in &mut self.web {
                    if web_host.host.contains(&host) {
                        /*
                            known: in new host, known in old host
                            unknown: in new host, unknown in old host
                            other: not in new host but in old host
                        */
                        let mut known_hosts: Vec<String> = Vec::new();
                        // unknown_hosts is left in original "hosts"
                        let mut other_hosts: Vec<String> = Vec::new();
                        let mut known_interfaces: Vec<ServerInterface> = Vec::new();
                        let mut unknown_interfaces: Vec<ServerInterface> = Vec::new();
                        let mut other_interfaces: Vec<ServerInterface> = Vec::new();
                        for h in &web_host.host {
                            let mut known_id = None;
                            'findhost: for id in 0..hosts.len() {
                                if h == &hosts[id] {
                                    known_hosts.push(h.clone());
                                    known_id = Some(id);
                                    break 'findhost;
                                }
                            }
                            if let Some(id) = known_id {
                                hosts.remove(id);
                            } else {
                                other_hosts.push(h.clone());
                            }
                        }
                        for i in 0..interfaces.len() {
                            let v = &interfaces[i];
                            if web_host.interface.contains(v) {
                                known_interfaces.push(v.clone());
                            } else {
                                unknown_interfaces.push(v.clone());
                            }
                        }
                        for v in &web_host.interface {
                            if !known_interfaces.contains(&v) {
                                other_interfaces.push(v.clone());
                            }
                        }
                        test_println!("KH {:?}", &known_hosts);
                        test_println!("UH {:?}", &hosts);
                        test_println!("OH {:?}", &other_hosts);
                        test_println!("KI {:?}", &known_interfaces);
                        test_println!("UI {:?}", &unknown_interfaces);
                        test_println!("OI {:?}", &other_interfaces);

                        if known_hosts.len() == 0 || known_interfaces.len() == 0 {
                            test_println!("Not current node, skipping");
                            hosts.extend(known_hosts);  // restore hosts in pair
                            test_println!("Current pair: ({:?}, {:?})", hosts, interfaces_for_all_hosts);
                            continue 'host_search_loop;
                        }

                        // logics to clear other_hosts (split web_host)
                        if other_hosts.len() > 0 {
                            test_println!("Host split {:?} KH={:?}, OH={:?}", web_host, known_hosts, other_hosts);
                            let mut new_host = web_host.clone();
                            new_host.host = other_hosts;
                            new_hosts.push(new_host);
                            
                            web_host.host = known_hosts;

                            test_println!("Split result: {:?} + {:?}", web_host, new_hosts);
                        }

                        // unknown hosts are left

                        // logics to clear other_interfaces (split web_host)
                        if other_interfaces.len() > 0 {
                            test_println!("Interface split {:?} KI={:?}, OI={:?}", web_host, known_interfaces, other_interfaces);
                            let mut new_host = web_host.clone();
                            new_host.interface = other_interfaces;
                            new_hosts.push(new_host);

                            web_host.interface = known_interfaces;

                            test_println!("Split result: {:?} + {:?}", web_host, new_hosts);
                        }

                        // logics to clear unknown_interfaces (leave)
                        if unknown_interfaces.len() > 0 {
                            interfaces.clear();
                            interfaces.extend(unknown_interfaces);
                        }
                        
                        // logics to clear known_interfaces
                        test_println!("Overwrite on {:?}", web_host);
                        if let Some(loc) = &inst.location {
                            execute_overwrite_policy!(policy, web_host.subservers.contains_key(loc), {
                                web_host.subservers.insert(loc.clone(), inst.descriptor.clone());
                            }, "Cannot overwrite existed server");
                        } else {
                            execute_overwrite_policy!(policy, web_host.server.is_some(), {
                                web_host.server = Some(inst.descriptor.clone());
                            }, "Cannot overwrite existed server");
                        }
                    }
                }
                self.web.extend(new_hosts);

                if interfaces.len() > 0 && interfaces.len() != interfaces_for_all_hosts.len() {
                    test_println!("Add new pair ({:?}, {:?})", host, interfaces);
                    new_pairs.push((vec![host.clone()], interfaces));
                } else {
                    test_println!("Preserve pair ({:?}, {:?})", hosts, interfaces_for_all_hosts);
                    i += 1;
                }

                test_println!("> Searched host {:?} self.web {:?}", host, self.web);
            }

            if hosts.len() > 0 {
                test_println!("Creating host {:?} interface {:?}", hosts, interfaces_for_all_hosts);
                let mut server = WebServer {
                    host: hosts.clone(),
                    interface: interfaces_for_all_hosts.clone(),
                    subservers: BTreeMap::new(),
                    server: None
                };
                if let Some(loc) = &inst.location {
                    server.subservers.insert(loc.clone(), inst.descriptor.clone());
                } else {
                    server.server = Some(inst.descriptor.clone());
                }
                self.web.push(server);
            }

            pairs.extend(new_pairs);
            pairs.remove(0);
            test_println!(">> Pairs after processing {:?}", &pairs);
        }
        test_println!(">>> Output {:?}", self.web);
        Ok(self)
    }
    fn clear(&mut self) {
        self.web.clear();
    }

    fn get_web_servers(&self) -> &Vec<WebServer> {
        &self.web
    }
}


/*
impl NginxHttpConfig for Registry {
    type Err = Void;
    fn to_nginx_http_config() -> Result<String, Self::Err> {
        Ok(String::from("http {\n\n") + self.to_nginx_server_blocks()? + "\n}\n")
    }
    fn to_nginx_server_blocks() -> Result<String, Self::Err> {
        Ok("# Unimplemented!".to_owned())
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct NullBackend {
        key: String,
    }
    impl BackendDescriptor for NullBackend {
        fn get_key(&self) -> String {
            self.key.clone()
        }
        fn to_backend_config(&self) -> Result<String, Box<dyn Error>> {
            Ok(format!("return 200 \"Hello world\"; # {:?}", self))
        }
    }


	#[test]
	fn registry_add_server_test_point_host_add_remove() {
        let mut reg: Registry = std::default::Default::default();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80]}]");

        reg.clear();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");
    }


	#[test]
	fn registry_add_server_test_point_host_separation() {
        let mut reg: Registry = std::default::Default::default();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host2".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\", \"host2\"], interface=[Http:80]}]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: Some("/test".to_owned()),
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80]}, WebServer {host=[\"host2\"], interface=[Http:80]}]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host3".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: Some("/test2".to_owned()),
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80]}, WebServer {host=[\"host2\"], interface=[Http:80]}, WebServer {host=[\"host3\"], interface=[Http:80]}]");
    }


	#[test]
	fn registry_add_server_test_point_interface_separation() {
        let mut reg: Registry = std::default::Default::default();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 81, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80, Http:81]}]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: Some("/test".to_owned()),
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80]}, WebServer {host=[\"host1\"], interface=[Http:81]}]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 82, attr: ServerInterfaceAttribute::Http },
            ],
            location: Some("/test2".to_owned()),
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80]}, WebServer {host=[\"host1\"], interface=[Http:81]}, WebServer {host=[\"host1\"], interface=[Http:82]}]");
    }

	#[test]
	fn registry_add_server_test_point_illegal_input() {
        let mut reg: Registry = std::default::Default::default();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");

        assert_eq!(match reg.add_server(&WebServerInstance {
            host: vec![],
            interface: vec![ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http }],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Error) {
            Err(e) => format!("{:?}", e),
            Ok(_) => panic!("Exception untriggered"),
        }, "Custom { kind: InvalidData, error: \"host is empty list\" }");

        assert_eq!(match reg.add_server(&WebServerInstance {
            host: vec!["aha".to_owned()],
            interface: vec![],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Error) {
            Err(e) => format!("{:?}", e),
            Ok(_) => panic!("Exception untriggered"),
        }, "Custom { kind: InvalidData, error: \"interface is empty list\" }");
    }

	#[test]
	fn registry_add_server_test_point_overwrite_policy() {
        let mut reg: Registry = std::default::Default::default();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host2".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 8080, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "waka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\", \"host2\"], interface=[Http:80, Http:8080]}]");

        assert_eq!(match reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host2".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 8080, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Error) {
            Err(e) => format!("{:?}", e),
            Ok(_) => panic!("Exception untriggered"),
        }, "Custom { kind: AlreadyExists, error: \"Cannot overwrite existed server\" }");

        assert_eq!(match reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 8080, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Error) {
            Err(e) => format!("{:?}", e),
            Ok(_) => panic!("Exception untriggered"),
        }, "Custom { kind: AlreadyExists, error: \"Cannot overwrite existed server\" }");

        assert_eq!(match reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host2".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Error) {
            Err(e) => format!("{:?}", e),
            Ok(_) => panic!("Exception untriggered"),
        }, "Custom { kind: AlreadyExists, error: \"Cannot overwrite existed server\" }");

        assert_eq!(match reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Error) {
            Err(e) => format!("{:?}", e),
            Ok(_) => panic!("Exception untriggered"),
        }, "Custom { kind: AlreadyExists, error: \"Cannot overwrite existed server\" }");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Ignore).unwrap();

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "wakaka".to_owned() })
        }, OverwritePolicy::Overwrite).unwrap();
    }

	#[test]
	fn registry_add_server_test_point_complex_separation() {
        let mut reg: Registry = std::default::Default::default();
        
        assert_eq!(format!("{:?}", reg.get_web_servers()), "[]");

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host2".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 8080, attr: ServerInterfaceAttribute::Http },
            ],
            location: None,
            descriptor: Arc::new(NullBackend { key: "waka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        reg.add_server(&WebServerInstance {
            host: vec![
                "host1".to_owned(),
                "host3".to_owned(),
            ],
            interface: vec![
                ServerInterface { port: 80, attr: ServerInterfaceAttribute::Http },
                ServerInterface { port: 443, attr: ServerInterfaceAttribute::Https },
            ],
            location: Some("/test".to_owned()),
            descriptor: Arc::new(NullBackend { key: "wakakaka".to_owned() })
        }, OverwritePolicy::Error).unwrap();

        assert_eq!(format!("{:?}", reg.get_web_servers()), "[WebServer {host=[\"host1\"], interface=[Http:80]}, WebServer {host=[\"host2\"], interface=[Http:80, Http:8080]}, WebServer {host=[\"host1\"], interface=[Http:8080]}, WebServer {host=[\"host3\"], interface=[Http:80, Https:443]}, WebServer {host=[\"host1\"], interface=[Https:443]}]");
	}
}
