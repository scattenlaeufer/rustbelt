//! A device to device file transfer program written in Rust

extern crate clap;

use clap::{crate_authors, crate_version, App, Arg};
use std::path::Path;

fn main() {
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
            Arg::with_name("network device")
                .short("n")
                .long("device")
                .value_name("NETWORK_DEVICE")
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        assert_eq!(1 + 1, 2);
    }
}
