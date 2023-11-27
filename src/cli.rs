use lazy_static::lazy_static;
use std::sync::Arc;

#[derive(Debug)]
struct Manager<'a> {
    clap_matches: clap::ArgMatches<'a>,
}

lazy_static! {
    static ref MANAGER: Arc<Manager<'static>> = Arc::new(Manager::new());
}

impl Manager<'_> {
    fn new() -> Self {
        Self {
            clap_matches: get_clap_matches(),
        }
    }
}

pub fn init() {
    MANAGER.as_ref();
}

pub fn is_verbose() -> bool {
    return MANAGER.as_ref().clap_matches.is_present("verbose");
}

pub fn mavlink_connection_string() -> &'static str {
    return MANAGER.as_ref().clap_matches.value_of("connect").unwrap();
}

pub fn server_address() -> &'static str {
    return MANAGER.as_ref().clap_matches.value_of("server").unwrap();
}

pub fn default_api_version() -> u8 {
    return MANAGER
        .as_ref()
        .clap_matches
        .value_of("default-api-version")
        .unwrap()
        .parse::<u8>()
        .unwrap();
}

pub fn mavlink_version() -> u8 {
    return MANAGER
        .as_ref()
        .clap_matches
        .value_of("mavlink")
        .unwrap()
        .parse::<u8>()
        .unwrap();
}

pub fn mavlink_system_and_component_id() -> (u8, u8) {
    let system_id = MANAGER
        .as_ref()
        .clap_matches
        .value_of("system_id")
        .unwrap()
        .parse::<u8>()
        .expect("System ID should be a value between 1-255.");

    let component_id = MANAGER
        .as_ref()
        .clap_matches
        .value_of("component_id")
        .unwrap()
        .parse::<u8>()
        .expect("Component ID should be a value between 1-255.");

    (system_id, component_id)
}

//TODO: Move to the top
fn get_clap_matches<'a>() -> clap::ArgMatches<'a> {
    let version = format!(
        "{} ({})",
        env!("VERGEN_GIT_SEMVER"),
        env!("VERGEN_BUILD_TIMESTAMP")
    );

    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(version.as_str())
        .about("MAVLink to REST API!")
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            clap::Arg::with_name("connect")
                .short("c")
                .long("connect")
                .value_name("TYPE:<IP/SERIAL>:<PORT/BAUDRATE>")
                .help("Sets the mavlink connection string")
                .takes_value(true)
                .default_value("udpin:0.0.0.0:14550"),
        )
        .arg(
            clap::Arg::with_name("server")
                .short("s")
                .long("server")
                .value_name("IP:PORT")
                .help("Sets the IP and port that the rest server will be provided")
                .takes_value(true)
                .default_value("0.0.0.0:8088"),
        )
        .arg(
            clap::Arg::with_name("mavlink")
                .long("mavlink")
                .value_name("VERSION")
                .help("Sets the mavlink version used to communicate")
                .takes_value(true)
                .possible_values(&["1", "2"])
                .default_value("2"),
        )
        .arg(
            clap::Arg::with_name("system_id")
                .long("system-id")
                .value_name("SYSTEM_ID")
                .help("Sets system ID for this service.")
                .takes_value(true)
                .default_value("255"),
        )
        .arg(
            clap::Arg::with_name("component_id")
                .long("component-id")
                .value_name("COMPONENT_ID")
                .help("Sets the component ID for this service, for more information, check: https://mavlink.io/en/messages/common.html#MAV_COMPONENT")
                .takes_value(true)
                .default_value("0"),
        )
        .arg(
            clap::Arg::with_name("default-api-version")
                .long("default-api-version")
                .value_name("DEFAULT_API_VERSION")
                .help("Sets the default version used by the REST API, this will remove the prefix used by its path.")
                .takes_value(true)
                .possible_values(&["1"])
                .default_value("1"),
        )
        .arg(
            clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Be verbose")
                .takes_value(false),
        );

    return matches.get_matches();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_arguments() {
        assert!(!is_verbose());
        assert_eq!(mavlink_connection_string(), "udpin:0.0.0.0:14550");
        assert_eq!(server_address(), "0.0.0.0:8088");
        assert_eq!(mavlink_version(), 2);
        assert_eq!(default_api_version(), 1);
    }
}
