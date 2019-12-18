//! A device to device file transfer program written in Rust

extern crate clap;
extern crate colored;
extern crate ipnetwork;
extern crate pnet;

use clap::{crate_authors, crate_version, App, Arg};
use colored::Colorize;
use pnet::datalink;
use qrcode::QrCode;
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::Path;

use std::convert::Infallible;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

async fn hello(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello World!")))
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    println!("Shutting down server");
}

#[tokio::main]
async fn run_http_server(
    socket: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello)) });

    let server = Server::bind(&socket).serve(make_svc);

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

#[derive(Debug)]
struct ChoiceError<T> {
    low: T,
    high: T,
}

impl<T> std::error::Error for ChoiceError<T> where T: fmt::Debug + fmt::Display {}

impl<T> fmt::Display for ChoiceError<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Not a valid choice. Must be between {} and {}",
            self.low, self.high
        )
    }
}

impl<T> ChoiceError<T> {
    fn new(low: T, high: T) -> ChoiceError<T> {
        ChoiceError { low, high }
    }
}

fn create_qr_code(data: String) -> String {
    QrCode::new(data)
        .unwrap()
        .render()
        .light_color(" ")
        .dark_color("█")
        .module_dimensions(2, 1)
        .build()
}

fn get_network_interfaces() -> HashMap<String, datalink::NetworkInterface> {
    let mut interface_map = HashMap::<String, datalink::NetworkInterface>::new();
    for interface in datalink::interfaces() {
        if !interface.ips.is_empty() {
            interface_map.insert(String::from(&interface.name), interface);
        }
    }
    interface_map
}

fn choose_number(
    message: String,
    choices: Vec<String>,
) -> Result<(usize, String), Box<dyn std::error::Error>> {
    println!("{}", message);
    for (index, choice) in choices.iter().enumerate() {
        println!("{} - {}", index, choice);
    }
    let mut choice_num_str = String::new();
    io::stdin().read_line(&mut choice_num_str).unwrap();

    let choice_num = match choice_num_str.trim().parse::<usize>() {
        Ok(n) => n,
        Err(e) => return Err(Box::new(e)),
    };

    if choice_num >= choices.len() {
        Err(Box::new(ChoiceError::new(0, choices.len() - 1)))
    } else {
        Ok((choice_num, choices[choice_num].clone()))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("rustbelt")
        .author(crate_authors!())
        .version(crate_version!())
        .arg(
            Arg::with_name("PATH")
                .required_unless("receive")
                .validator(|s: String| {
                    if Path::new(&s).exists() {
                        Ok(())
                    } else {
                        Err(String::from("File or path does not exist"))
                    }
                })
                .help("Path to a file or directory to be transferred."),
        )
        .arg(
            Arg::with_name("receive")
                .help("Receive data from a source instead of sending it")
                .short("r")
                .long("receive"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Produce more verbose output. Multiple usage for more verbose output"),
        )
        .arg(
            Arg::with_name("network interface")
                .short("i")
                .long("interface")
                .value_name("NETWORK_INTERFACE")
                .validator(|d: String| {
                    for (name, _) in get_network_interfaces() {
                        if d == name {
                            return Ok(());
                        }
                    }
                    Err(String::from("Device not found!"))
                })
                .help("The network device over which the web server will run"),
        )
        .arg(
            Arg::with_name("domain")
                .short("d")
                .long("domain")
                .value_name("DOMAIN")
                .help("The domain, the web server should be served on"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("3000")
                .validator(|p: String| match &p.parse::<u16>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err(String::from("Must be a integer between 0 and 65536")),
                })
                .help("Port the web server listens on"),
        )
        .get_matches();

    if matches.occurrences_of("verbose") >= 1 {
        println!("Arguments: {:?}", matches);
    }

    let interface_map = get_network_interfaces();
    let network_interface = if matches.occurrences_of("network interface") == 1 {
        match interface_map.get(matches.value_of("network interface").unwrap()) {
            Some(i) => i,
            None => panic!("Network interface not found!"),
        }
    } else {
        println!("Found network interfaces, choose one:");
        let mut interface_names = interface_map.keys().cloned().collect::<Vec<String>>();
        interface_names.sort();
        for (index, device_name) in interface_names.iter().enumerate() {
            println!("{} - {}", index, device_name);
        }
        let mut interface_num_str = String::new();
        io::stdin().read_line(&mut interface_num_str).unwrap();

        let interface_num = match interface_num_str.trim().parse::<usize>() {
            Ok(n) => n,
            Err(e) => return Err(Box::new(e)),
        };

        if interface_num >= interface_map.len() {
            return Err(Box::new(ChoiceError::new(0, interface_map.len() - 1)));
        }
        &interface_map[&interface_names[interface_num]]
    };

    if matches.occurrences_of("verbose") >= 1 {
        println!("{:#?}", network_interface);
    }

    let (ipaddr_count, ipaddr_string) = choose_number(
        String::from("Choose an IP address:"),
        network_interface
            .ips
            .iter()
            .map(|ip| match ip {
                ipnetwork::IpNetwork::V4(ipv4) => format!(
                    "{}.{}.{}.{}",
                    ipv4.ip().octets()[0],
                    ipv4.ip().octets()[1],
                    ipv4.ip().octets()[2],
                    ipv4.ip().octets()[3]
                ),
                ipnetwork::IpNetwork::V6(ipv6) => format!(
                    "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                    ipv6.ip().segments()[0],
                    ipv6.ip().segments()[1],
                    ipv6.ip().segments()[2],
                    ipv6.ip().segments()[3],
                    ipv6.ip().segments()[4],
                    ipv6.ip().segments()[5],
                    ipv6.ip().segments()[6],
                    ipv6.ip().segments()[7]
                ),
            })
            .collect(),
    )?;

    let (url, socket) = match network_interface.ips[ipaddr_count] {
        ipnetwork::IpNetwork::V4(v4) => (
            format!(
                "http://{}:{}",
                ipaddr_string,
                matches.value_of("port").unwrap().parse::<u16>()?
            ),
            std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
                v4.ip(),
                matches.value_of("port").unwrap().parse::<u16>()?,
            )),
        ),
        ipnetwork::IpNetwork::V6(v6) => (
            format!(
                "http://[{}]:{}",
                ipaddr_string,
                matches.value_of("port").unwrap().parse::<u16>()?
            ),
            std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
                v6.ip(),
                matches.value_of("port").unwrap().parse::<u16>()?,
                0,
                0,
            )),
        ),
    };
    println!("Listening on {}", url);

    for split in create_qr_code(url).split('\n') {
        println!("{}", split.black().on_white());
    }

    match run_http_server(socket) {
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn test_create_qr_code() {
        let test_code = "                                                          \n                                                          \n                                                          \n                                                          \n        ██████████████      ██      ██████████████        \n        ██          ██  ██  ██  ██  ██          ██        \n        ██  ██████  ██        ██    ██  ██████  ██        \n        ██  ██████  ██    ████      ██  ██████  ██        \n        ██  ██████  ██  ████  ████  ██  ██████  ██        \n        ██          ██    ██  ██    ██          ██        \n        ██████████████  ██  ██  ██  ██████████████        \n                          ████                            \n        ██  ██  ██  ██      ██  ██      ██    ██          \n            ████████  ██    ████  ██  ██      ████        \n        ██  ██      ████████████  ██████  ████████        \n              ██████    ████████████  ████    ██          \n        ██  ██  ██  ██    ██████  ██████  ██  ████        \n                        ██          ██    ██    ██        \n        ██████████████    ██    ██      ████  ████        \n        ██          ██      ██      ██        ██          \n        ██  ██████  ██  ██████  ██  ██  ████  ████        \n        ██  ██████  ██      ████  ██  ██      ██          \n        ██  ██████  ██  ████████  ██████    ██  ██        \n        ██          ██      ████████  ██████  ██          \n        ██████████████  ████████  ██████    ██████        \n                                                          \n                                                          \n                                                          \n                                                          ";
        assert_eq!(test_code, create_qr_code(String::from("test")));
    }
}
