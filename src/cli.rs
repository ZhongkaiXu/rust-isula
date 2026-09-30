pub const DEFAULT_HOST: &str = "unix:///var/run/isulad.sock";

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedOption {
    pub name: &'static str,
    pub value: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Invocation {
    pub name: &'static str,
    pub host: String,
    pub options: Vec<ParsedOption>,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HelpTopic {
    Root,
    Group(&'static str),
    Command(&'static str),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Help(HelpTopic),
    Version,
    Invoke(Invocation),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CliError {
    pub message: String,
    pub exit_code: i32,
}

impl CliError {
    fn command(message: String) -> Self {
        Self {
            message,
            exit_code: 1,
        }
    }

    fn option(message: String) -> Self {
        Self {
            message,
            exit_code: 125,
        }
    }
}

impl From<String> for CliError {
    fn from(message: String) -> Self {
        Self::option(message)
    }
}

struct CommandSpec {
    name: &'static str,
    description: &'static str,
    usage: &'static str,
}

const fn command(
    name: &'static str,
    description: &'static str,
    usage: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        description,
        usage,
    }
}

// Names and short descriptions follow the iSula 2.1.5 client on the test server.
const COMMANDS: &[CommandSpec] = &[
    command(
        "attach",
        "Attach to a running container",
        "attach [OPTIONS] CONTAINER",
    ),
    command(
        "cp",
        "Copy files between a container and the local filesystem",
        "cp [OPTIONS] SRC_PATH DEST_PATH",
    ),
    command(
        "create",
        "Create a new container",
        "create [OPTIONS] IMAGE [COMMAND] [ARG...]",
    ),
    command(
        "events",
        "Get real time events from the server",
        "events [OPTIONS]",
    ),
    command(
        "exec",
        "Run a command in a running container",
        "exec [OPTIONS] CONTAINER COMMAND [ARG...]",
    ),
    command("export", "Export a container", "export [OPTIONS] CONTAINER"),
    command("images", "List images", "images [OPTIONS]"),
    command(
        "import",
        "Import a tarball as a filesystem image",
        "import FILE REPOSITORY[:TAG]",
    ),
    command("info", "Display system-wide information", "info [OPTIONS]"),
    command(
        "inspect",
        "Return information on a container or image",
        "inspect [OPTIONS] CONTAINER|IMAGE",
    ),
    command(
        "kill",
        "Kill one or more running containers",
        "kill [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "load",
        "Load an image from a tar archive",
        "load [OPTIONS] --input=FILE",
    ),
    command("login", "Log in to a registry", "login [OPTIONS] SERVER"),
    command("logout", "Log out from a registry", "logout SERVER"),
    command(
        "logs",
        "Fetch the logs of a container",
        "logs [OPTIONS] CONTAINER",
    ),
    command(
        "pause",
        "Pause processes in containers",
        "pause [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "port",
        "List port mappings of a container",
        "port CONTAINER [PRIVATE_PORT[/PROTO]]",
    ),
    command("ps", "List containers", "ps [OPTIONS]"),
    command(
        "pull",
        "Pull an image from a registry",
        "pull [OPTIONS] NAME[:TAG]",
    ),
    command("rename", "Rename a container", "rename OLD_NAME NEW_NAME"),
    command(
        "restart",
        "Restart containers",
        "restart [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "rm",
        "Remove containers",
        "rm [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command("rmi", "Remove images", "rmi [OPTIONS] IMAGE [IMAGE...]"),
    command(
        "run",
        "Run a command in a new container",
        "run [OPTIONS] IMAGE [COMMAND] [ARG...]",
    ),
    command(
        "search",
        "Search the registry for images",
        "search [OPTIONS] TERM",
    ),
    command(
        "start",
        "Start stopped containers",
        "start [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "stats",
        "Show container resource usage",
        "stats [OPTIONS] [CONTAINER...]",
    ),
    command(
        "stop",
        "Stop containers",
        "stop [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "tag",
        "Create a new image tag",
        "tag SOURCE_IMAGE TARGET_IMAGE",
    ),
    command(
        "top",
        "Display processes in a container",
        "top [OPTIONS] CONTAINER [ps OPTIONS]",
    ),
    command(
        "unpause",
        "Resume processes in containers",
        "unpause [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "update",
        "Update container configuration",
        "update [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "version",
        "Display information about isula",
        "version [OPTIONS]",
    ),
    command(
        "wait",
        "Wait for containers to stop",
        "wait [OPTIONS] CONTAINER [CONTAINER...]",
    ),
    command(
        "network create",
        "Create a network",
        "network create [OPTIONS] [NETWORK]",
    ),
    command(
        "network inspect",
        "Inspect networks",
        "network inspect [OPTIONS] NETWORK [NETWORK...]",
    ),
    command("network ls", "List networks", "network ls [OPTIONS]"),
    command(
        "network rm",
        "Remove networks",
        "network rm [OPTIONS] NETWORK [NETWORK...]",
    ),
    command("volume ls", "List volumes", "volume ls [OPTIONS]"),
    command(
        "volume prune",
        "Remove unused local volumes",
        "volume prune [OPTIONS]",
    ),
    command(
        "volume rm",
        "Remove volumes",
        "volume rm [OPTIONS] VOLUME [VOLUME...]",
    ),
];

#[derive(Clone, Copy)]
struct OptionSpec {
    long: &'static str,
    short: Option<char>,
    takes_value: bool,
    description: &'static str,
}

const fn flag(long: &'static str, short: Option<char>, description: &'static str) -> OptionSpec {
    OptionSpec {
        long,
        short,
        takes_value: false,
        description,
    }
}

const fn value(long: &'static str, short: Option<char>, description: &'static str) -> OptionSpec {
    OptionSpec {
        long,
        short,
        takes_value: true,
        description,
    }
}

const COMMON_OPTIONS: &[OptionSpec] = &[
    flag("debug", Some('D'), "Enable debug mode"),
    value("host", Some('H'), "Daemon socket to connect to"),
];

const LOAD_OPTIONS: &[OptionSpec] = &[
    value("input", Some('i'), "Read from a tar archive"),
    value("tag", None, "Image name and optional tag"),
];

const IMAGES_OPTIONS: &[OptionSpec] = &[
    value("filter", Some('f'), "Filter output by condition"),
    flag("quiet", Some('q'), "Only display image names"),
];

const PS_OPTIONS: &[OptionSpec] = &[
    flag("all", Some('a'), "Display all containers"),
    value("filter", Some('f'), "Filter output by condition"),
    value("format", None, "Format output"),
    value("last", Some('n'), "Display the last n containers"),
    flag("latest", Some('l'), "Display the latest container"),
    flag("no-trunc", None, "Do not truncate output"),
    flag("quiet", Some('q'), "Only display container IDs"),
];

fn options_for(command: &str) -> &'static [OptionSpec] {
    match command {
        "load" => LOAD_OPTIONS,
        "images" => IMAGES_OPTIONS,
        "ps" => PS_OPTIONS,
        _ => &[],
    }
}

fn find_command(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|command| command.name == name)
}

fn find_long_option(command: &str, name: &str) -> Option<OptionSpec> {
    COMMON_OPTIONS
        .iter()
        .chain(options_for(command))
        .find(|option| option.long == name)
        .copied()
}

fn find_short_option(command: &str, letter: char) -> Option<OptionSpec> {
    COMMON_OPTIONS
        .iter()
        .chain(options_for(command))
        .find(|option| option.short == Some(letter))
        .copied()
}

fn option_value(
    args: &[String],
    index: &mut usize,
    inline: Option<&str>,
    name: &str,
) -> Result<String, String> {
    if let Some(value) = inline {
        if !value.is_empty() {
            return Ok(value.to_string());
        }
    } else {
        *index += 1;
        if let Some(value) = args.get(*index) {
            return Ok(value.clone());
        }
    }
    Err(format!("option {name} requires a value"))
}

fn save_option(
    host: &mut String,
    options: &mut Vec<ParsedOption>,
    spec: OptionSpec,
    value: Option<String>,
) {
    if spec.long == "host" {
        *host = value.expect("host requires a value");
    } else {
        options.push(ParsedOption {
            name: spec.long,
            value,
        });
    }
}

fn check_host(host: &str) -> Result<(), String> {
    if let Some(path) = host.strip_prefix("unix://") {
        if path.starts_with('/') && path.len() > 1 {
            return Ok(());
        }
    }
    if let Some(address) = host.strip_prefix("tcp://") {
        if address.contains(':') && !address.ends_with(':') {
            return Ok(());
        }
    }
    Err(format!("invalid daemon address: {host}"))
}

fn check_args(command: &str, options: &[ParsedOption], args: &[String]) -> Result<(), String> {
    match command {
        "pull" if args.len() != 1 || args[0].is_empty() => {
            Err("pull requires one image name".to_string())
        }
        "load" if !args.is_empty() => Err("load takes no positional arguments".to_string()),
        "load" if !options.iter().any(|option| option.name == "input") => {
            Err("load requires -i or --input".to_string())
        }
        "ps" | "version" | "info" | "network ls" | "volume ls" | "volume prune"
            if !args.is_empty() =>
        {
            Err(format!("{command} takes no positional arguments"))
        }
        _ => Ok(()),
    }
}

pub fn parse(args: &[String], env_host: Option<&str>) -> Result<Action, CliError> {
    if args.is_empty() || args[0] == "--help" {
        return Ok(Action::Help(HelpTopic::Root));
    }
    if args[0] == "--version" {
        return Ok(Action::Version);
    }

    let (name, option_start) = if args[0] == "network" || args[0] == "volume" {
        if args.len() == 1 || args[1] == "--help" {
            let group = if args[0] == "network" {
                "network"
            } else {
                "volume"
            };
            return Ok(Action::Help(HelpTopic::Group(group)));
        }
        (format!("{} {}", args[0], args[1]), 2)
    } else {
        (args[0].clone(), 1)
    };

    let command =
        find_command(&name).ok_or_else(|| CliError::command(format!("unknown command: {name}")))?;
    let mut host = env_host
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_HOST)
        .to_string();
    let mut options = Vec::new();
    let mut positionals = Vec::new();
    let mut index = option_start;

    while index < args.len() {
        let token = &args[index];
        if token == "--" {
            positionals.extend_from_slice(&args[index + 1..]);
            break;
        }
        if !token.starts_with('-') || token == "-" {
            positionals.extend_from_slice(&args[index..]);
            break;
        }

        if let Some(long) = token.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (long, None),
            };
            if name == "help" && inline.is_none() {
                return Ok(Action::Help(HelpTopic::Command(command.name)));
            }
            let spec = find_long_option(command.name, name)
                .ok_or_else(|| format!("unknown option: --{name}"))?;
            let value = if spec.takes_value {
                Some(option_value(args, &mut index, inline, name)?)
            } else if inline.is_some() {
                return Err(CliError::option(format!(
                    "option --{name} does not take a value"
                )));
            } else {
                None
            };
            save_option(&mut host, &mut options, spec, value);
        } else {
            let mut letters = token[1..].chars();
            while let Some(letter) = letters.next() {
                let spec = find_short_option(command.name, letter)
                    .ok_or_else(|| format!("unknown option: -{letter}"))?;
                let value = if spec.takes_value {
                    let rest = letters.as_str().trim_start_matches('=');
                    Some(option_value(
                        args,
                        &mut index,
                        if rest.is_empty() { None } else { Some(rest) },
                        spec.long,
                    )?)
                } else {
                    None
                };
                save_option(&mut host, &mut options, spec, value);
                if spec.takes_value {
                    break;
                }
            }
        }
        index += 1;
    }

    check_host(&host)?;
    check_args(command.name, &options, &positionals).map_err(CliError::command)?;

    Ok(Action::Invoke(Invocation {
        name: command.name,
        host,
        options,
        args: positionals,
    }))
}

pub fn print_help(topic: HelpTopic) {
    match topic {
        HelpTopic::Root => {
            println!("USAGE:\n    risula <command> [args...]\n");
            println!("MANAGEMENT COMMANDS:\n    network    Manage networks\n    volume     Manage volumes\n");
            println!("COMMANDS:");
            for command in COMMANDS
                .iter()
                .filter(|command| !command.name.contains(' '))
            {
                println!("    {:<10} {}", command.name, command.description);
            }
            println!("\nCOMMON OPTIONS:\n    --help      Print usage\n    -H, --host  Daemon socket\n    --version   Print version");
        }
        HelpTopic::Group(group) => {
            println!("USAGE:\n    risula {group} <command> [args...]\n");
            println!("COMMANDS:");
            let prefix = format!("{group} ");
            for command in COMMANDS
                .iter()
                .filter(|command| command.name.starts_with(&prefix))
            {
                println!(
                    "    {:<10} {}",
                    command.name.strip_prefix(&prefix).unwrap_or(command.name),
                    command.description
                );
            }
        }
        HelpTopic::Command(name) => {
            let command = find_command(name).expect("known command");
            println!(
                "Usage: risula {}\n\n{}\n",
                command.usage, command.description
            );
            println!("OPTIONS:");
            for option in COMMON_OPTIONS.iter().chain(options_for(name)) {
                let short = option
                    .short
                    .map(|letter| format!("-{letter}, "))
                    .unwrap_or_default();
                let suffix = if option.takes_value { " VALUE" } else { "" };
                println!(
                    "    {short}--{}{}    {}",
                    option.long, suffix, option.description
                );
            }
            println!("    --help    Print usage");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn dispatches_commands_and_groups() {
        assert_eq!(
            parse(&words("network"), None),
            Ok(Action::Help(HelpTopic::Group("network")))
        );
        let Action::Invoke(command) = parse(&words("network ls"), None).unwrap() else {
            panic!("expected command");
        };
        assert_eq!(command.name, "network ls");
        assert!(parse(&words("network missing"), None).is_err());
    }

    #[test]
    fn host_flag_overrides_environment() {
        let Action::Invoke(command) = parse(
            &words("images -H unix:///tmp/isulad.sock"),
            Some("unix:///other.sock"),
        )
        .unwrap() else {
            panic!("expected command");
        };
        assert_eq!(command.host, "unix:///tmp/isulad.sock");
        let Action::Invoke(command) = parse(&words("images"), Some("unix:///other.sock")).unwrap()
        else {
            panic!("expected command");
        };
        assert_eq!(command.host, "unix:///other.sock");
    }

    #[test]
    fn short_flags_and_attached_values() {
        let Action::Invoke(command) =
            parse(&words("ps -aq -H=unix:///tmp/isulad.sock"), None).unwrap()
        else {
            panic!("expected command");
        };
        assert_eq!(command.host, "unix:///tmp/isulad.sock");
        assert_eq!(command.options.len(), 2);
        assert_eq!(command.options[0].name, "all");
        assert_eq!(command.options[1].name, "quiet");
    }

    #[test]
    fn validates_image_arguments() {
        assert_eq!(parse(&words("pull"), None).unwrap_err().exit_code, 1);
        assert_eq!(parse(&words("load"), None).unwrap_err().exit_code, 1);
        assert_eq!(
            parse(&words("pull --bad"), None).unwrap_err().exit_code,
            125
        );
        assert!(parse(&words("load -i archive.tar"), None).is_ok());
        assert!(parse(&words("images unexpected"), None).is_ok());
    }

    #[test]
    fn stops_parsing_after_double_dash() {
        let Action::Invoke(command) = parse(&words("pull -- -image"), None).unwrap() else {
            panic!("expected command");
        };
        assert_eq!(command.args, vec!["-image"]);
    }
}
