use colored::Colorize;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use pnet::datalink;
use qrcode::QrCode;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error;
use std::fmt;
use std::io;
use std::net;

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

enum IpString {
    V4(String),
    V6(String),
}

pub fn get_network_interfaces() -> HashMap<String, datalink::NetworkInterface> {
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

fn create_qr_code(data: String) -> String {
    QrCode::new(data)
        .unwrap()
        .render()
        .light_color(" ")
        .dark_color("█")
        .module_dimensions(2, 1)
        .build()
}

fn choose_ip(
    message: String,
    choices: Vec<IpString>,
) -> Result<(usize, IpString), Box<dyn error::Error>> {
    let (interface_num, ip_string) = choose_number(
        message,
        choices
            .iter()
            .map(|ip| match ip {
                IpString::V4(s) => s.clone(),
                IpString::V6(s) => s.clone(),
            })
            .collect(),
    )?;
    Ok((
        interface_num,
        match choices[interface_num] {
            IpString::V4(_) => IpString::V4(ip_string),
            IpString::V6(_) => IpString::V6(ip_string),
        },
    ))
}

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

fn get_network_socket(
    matches: &clap::ArgMatches,
) -> Result<(String, net::SocketAddr), Box<dyn error::Error>> {
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
        let (interface_num, _) = choose_number(
            String::from("Found network interfaces, choose one:"),
            interface_names.clone(),
        )?;

        &interface_map[&interface_names[interface_num]]
    };

    if matches.occurrences_of("verbose") >= 1 {
        println!("{:#?}", network_interface);
    }

    let (ipaddr_count, ipaddr_string) = choose_ip(
        String::from("Choose an IP address:"),
        network_interface
            .ips
            .iter()
            .map(|ip| match ip {
                ipnetwork::IpNetwork::V4(ipv4) => IpString::V4(format!(
                    "{}.{}.{}.{}",
                    ipv4.ip().octets()[0],
                    ipv4.ip().octets()[1],
                    ipv4.ip().octets()[2],
                    ipv4.ip().octets()[3]
                )),
                ipnetwork::IpNetwork::V6(ipv6) => IpString::V6(format!(
                    "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                    ipv6.ip().segments()[0],
                    ipv6.ip().segments()[1],
                    ipv6.ip().segments()[2],
                    ipv6.ip().segments()[3],
                    ipv6.ip().segments()[4],
                    ipv6.ip().segments()[5],
                    ipv6.ip().segments()[6],
                    ipv6.ip().segments()[7]
                )),
            })
            .collect(),
    )?;
    let socket = create_socket(
        network_interface.ips[ipaddr_count],
        matches.value_of("port").unwrap().parse::<u16>()?,
    )?;
    let url = create_url(
        ipaddr_string,
        matches.value_of("port").unwrap().parse::<u16>()?,
    )?;
    Ok((url, socket))
}

fn create_socket(
    ip: ipnetwork::IpNetwork,
    port: u16,
) -> Result<net::SocketAddr, Box<dyn error::Error>> {
    match ip {
        ipnetwork::IpNetwork::V4(v4) => Ok(std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            v4.ip(),
            port,
        ))),
        ipnetwork::IpNetwork::V6(v6) => Ok(std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
            v6.ip(),
            port,
            0,
            0,
        ))),
    }
}

fn create_url(ip: IpString, port: u16) -> Result<String, Box<dyn error::Error>> {
    match ip {
        IpString::V4(v4) => Ok(format!("http://{}:{}", v4, port)),
        IpString::V6(v6) => Ok(format!("http://[{}]:{}", v6, port)),
    }
}

pub fn run_rustbelt(matches: &clap::ArgMatches) -> Result<(), Box<dyn error::Error>> {
    let (url, socket) = get_network_socket(matches)?;

    println!("Listening on {}", url);

    for split in create_qr_code(url).split('\n') {
        println!("{}", split.black().on_white());
    }
    match run_http_server(socket) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_url_creation_v4(a: u8, b: u8, c: u8, d: u8) {
            let ip_string = format!("{}.{}.{}.{}", a, b, c, d);
            let url = match create_url(IpString::V4(ip_string.clone()), 0) {
                Ok(s) => s,
                Err(_) => panic!("failed for {}", ip_string),
            };
            prop_assert_eq!(format!("http://{}:0", ip_string), url);
        }

        #[test]
        fn test_url_creation_v6(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) {
            let ip_string = format!("{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}", a, b, c, d, e, f, g, h);
            let url = match create_url(IpString::V6(ip_string.clone()), 0) {
                Ok(s) => s,
                Err(_) => panic!("failed for {}", ip_string),
            };
            prop_assert_eq!(format!("http://[{}]:0", ip_string), url);
        }
    }

    #[test]
    fn test_create_qr_code() {
        let test_code = "                                                          \n                                                          \n                                                          \n                                                          \n        ██████████████      ██      ██████████████        \n        ██          ██  ██  ██  ██  ██          ██        \n        ██  ██████  ██        ██    ██  ██████  ██        \n        ██  ██████  ██    ████      ██  ██████  ██        \n        ██  ██████  ██  ████  ████  ██  ██████  ██        \n        ██          ██    ██  ██    ██          ██        \n        ██████████████  ██  ██  ██  ██████████████        \n                          ████                            \n        ██  ██  ██  ██      ██  ██      ██    ██          \n            ████████  ██    ████  ██  ██      ████        \n        ██  ██      ████████████  ██████  ████████        \n              ██████    ████████████  ████    ██          \n        ██  ██  ██  ██    ██████  ██████  ██  ████        \n                        ██          ██    ██    ██        \n        ██████████████    ██    ██      ████  ████        \n        ██          ██      ██      ██        ██          \n        ██  ██████  ██  ██████  ██  ██  ████  ████        \n        ██  ██████  ██      ████  ██  ██      ██          \n        ██  ██████  ██  ████████  ██████    ██  ██        \n        ██          ██      ████████  ██████  ██          \n        ██████████████  ████████  ██████    ██████        \n                                                          \n                                                          \n                                                          \n                                                          ";
        assert_eq!(test_code, create_qr_code(String::from("test")));
    }
}
