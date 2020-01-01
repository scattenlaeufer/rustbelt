//! A device to device file transfer program written in Rust

extern crate clap;
extern crate colored;
extern crate ipnetwork;
extern crate pnet;

use clap::{crate_authors, crate_version, App, Arg};
use rustbelt;
use std::path::Path;

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
                    for (name, _) in rustbelt::get_network_interfaces() {
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

    rustbelt::run_rustbelt(&matches)
}
