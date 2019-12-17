//! A device to device file transfer program written in Rust

extern crate clap;
extern crate colored;
extern crate pnet;

use clap::{crate_authors, crate_version, App, Arg};
use colored::Colorize;
use pnet::datalink;
use qrcode::QrCode;
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::Path;

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
                // TODO This should be changed to 443 once we introduce encryption
                .default_value("80")
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

    for split in create_qr_code(String::from("test string")).split('\n') {
        println!("{}", split.black().on_white());
    }

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
