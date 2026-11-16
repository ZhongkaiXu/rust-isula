mod create;

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
        "create [OPTIONS] --external-rootfs=PATH|IMAGE [COMMAND] [ARG...]",
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
        "inspect [OPTIONS] CONTAINER|IMAGE [CONTAINER|IMAGE...]",
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
    command(
        "rename",
        "Rename a container",
        "rename [OPTIONS] OLD_NAME NEW_NAME",
    ),
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
        "run [OPTIONS] ROOTFS|IMAGE [COMMAND] [ARG...]",
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
        "tag SOURCE_IMAGE[:TAG] TARGET_IMAGE[:TAG]",
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

const LOGIN_OPTIONS: &[OptionSpec] = &[
    value("username", Some('u'), "Username"),
    value("password", Some('p'), "Password"),
    flag("password-stdin", None, "Take the password from stdin"),
];

const IMAGES_OPTIONS: &[OptionSpec] = &[
    value("filter", Some('f'), "Filter output by condition"),
    flag("quiet", Some('q'), "Only display image names"),
];

const RMI_OPTIONS: &[OptionSpec] = &[flag("force", Some('f'), "Force removal of the image")];

const RM_OPTIONS: &[OptionSpec] = &[
    flag(
        "force",
        Some('f'),
        "Force the removal of a running container (uses SIGKILL)",
    ),
    flag(
        "volumes",
        Some('v'),
        "Remove anonymous volumes associated with the container",
    ),
];

const KILL_OPTIONS: &[OptionSpec] = &[value(
    "signal",
    Some('s'),
    "Signal to send to the container (default SIGKILL)",
)];

const STOP_OPTIONS: &[OptionSpec] = &[
    flag("force", Some('f'), "Stop by force killing"),
    value(
        "time",
        Some('t'),
        "Seconds to wait for stop before killing it (default 10)",
    ),
];

const VOLUME_LIST_OPTIONS: &[OptionSpec] = &[
    flag("debug", Some('D'), "Enable debug mode"),
    flag("quiet", Some('q'), "Only display volume names"),
];

const UPDATE_OPTIONS: &[OptionSpec] = &[
    flag("debug", Some('D'), "Enable debug mode"),
    value("cpu-shares", None, "CPU shares (relative weight)"),
    value("cpu-period", None, "Limit CPU CFS period"),
    value("cpu-quota", None, "Limit CPU CFS quota"),
    value("cpuset-cpus", None, "CPUs in which to allow execution"),
    value("cpuset-mems", None, "MEMs in which to allow execution"),
    value("kernel-memory", None, "Kernel memory limit"),
    value("memory", Some('m'), "Memory limit"),
    value("memory-reservation", None, "Memory soft limit"),
    value(
        "memory-swap",
        None,
        "Memory plus swap limit; -1 means unlimited swap",
    ),
    value(
        "restart",
        None,
        "Restart policy to apply when a container exits",
    ),
    value("blkio-weight", None, "Block IO relative weight"),
    value(
        "cpu-rt-period",
        None,
        "CPU real-time period in microseconds",
    ),
    value(
        "cpu-rt-runtime",
        None,
        "CPU real-time runtime in microseconds",
    ),
    value("cpus", None, "Number of CPUs"),
];

const EXPORT_OPTIONS: &[OptionSpec] = &[
    flag("debug", Some('D'), "Enable debug mode"),
    value("output", Some('o'), "Write to a file"),
];

const EVENTS_OPTIONS: &[OptionSpec] = &[
    flag("debug", Some('D'), "Enable debug mode"),
    value("name", Some('n'), "Name of the container"),
    value(
        "since",
        Some('S'),
        "Show all events created since this timestamp",
    ),
    value(
        "until",
        Some('U'),
        "Show all events created until this timestamp",
    ),
];

const LOGS_OPTIONS: &[OptionSpec] = &[
    flag("debug", Some('D'), "Enable debug mode"),
    flag("follow", Some('f'), "Follow log output"),
    flag("timestamps", Some('t'), "Show timestamps"),
    value(
        "tail",
        None,
        "Number of lines to show from the end of the logs",
    ),
];

const ATTACH_OPTIONS: &[OptionSpec] = &[flag("debug", Some('D'), "Enable debug mode")];

const EXEC_OPTIONS: &[OptionSpec] = &[
    flag("detach", Some('d'), "Run without attaching input or output"),
    flag("interactive", Some('i'), "Keep STDIN open"),
    flag("tty", Some('t'), "Allocate a pseudo-TTY"),
    value("env", Some('e'), "Set environment variables"),
    value("user", Some('u'), "Username or UID (optionally with group)"),
    value("workdir", None, "Working directory inside the container"),
];

const START_OPTIONS: &[OptionSpec] = &[flag("attach", Some('a'), "Attach STDOUT/STDERR")];

const RESTART_OPTIONS: &[OptionSpec] = &[value(
    "time",
    Some('t'),
    "Seconds to wait for stop before killing it (default 10)",
)];

const SEARCH_OPTIONS: &[OptionSpec] = &[
    value("limit", None, "Maximum number of results (default 25)"),
    value("filter", Some('f'), "Filter results by condition"),
    flag("no-trunc", None, "Do not truncate descriptions"),
    value("format", None, "Format output with field placeholders"),
];

const INSPECT_OPTIONS: &[OptionSpec] = &[
    value("format", Some('f'), "Format output using field paths"),
    value(
        "time",
        Some('t'),
        "Seconds to wait for inspect timeout (default 120)",
    ),
];

const STATS_OPTIONS: &[OptionSpec] = &[
    flag("all", Some('a'), "Show all containers"),
    flag("no-stream", None, "Display one sample"),
    flag("original", None, "Show original data"),
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

fn options_for(command: &str) -> impl Iterator<Item = &'static OptionSpec> {
    let options = match command {
        "create" | "run" => create::OPTIONS,
        "load" => LOAD_OPTIONS,
        "login" => LOGIN_OPTIONS,
        "images" => IMAGES_OPTIONS,
        "rmi" => RMI_OPTIONS,
        "rm" => RM_OPTIONS,
        "kill" => KILL_OPTIONS,
        "stop" => STOP_OPTIONS,
        "start" => START_OPTIONS,
        "exec" => EXEC_OPTIONS,
        "attach" | "cp" | "port" => ATTACH_OPTIONS,
        "logs" => LOGS_OPTIONS,
        "events" => EVENTS_OPTIONS,
        "export" => EXPORT_OPTIONS,
        "update" => UPDATE_OPTIONS,
        "volume ls" => VOLUME_LIST_OPTIONS,
        "restart" => RESTART_OPTIONS,
        "search" => SEARCH_OPTIONS,
        "ps" => PS_OPTIONS,
        "stats" => STATS_OPTIONS,
        "inspect" => INSPECT_OPTIONS,
        _ => &[],
    };
    const RUN_OPTIONS: &[OptionSpec] = &[flag(
        "detach",
        Some('d'),
        "Run container in background and print container ID",
    )];
    options
        .iter()
        .chain(if command == "run" { RUN_OPTIONS } else { &[] })
}

fn find_command(name: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|command| command.name == name)
}

fn accepts_bool_value(command: &str, option: &str) -> bool {
    if matches!(
        command,
        "create"
            | "run"
            | "stats"
            | "exec"
            | "logs"
            | "events"
            | "export"
            | "port"
            | "update"
            | "volume ls"
    ) && options_for(command).any(|spec| spec.long == option && !spec.takes_value)
    {
        return true;
    }
    matches!(
        (command, option),
        ("stop" | "rm", "force") | ("rm", "volumes") | ("start", "attach")
    )
}

pub(crate) fn parse_bool(value: Option<&str>, name: &str) -> Result<bool, String> {
    match value.unwrap_or("true") {
        "1" | "t" | "T" | "true" | "TRUE" | "True" => Ok(true),
        "0" | "f" | "F" | "false" | "FALSE" | "False" => Ok(false),
        value => Err(format!("invalid boolean value '{value}' for --{name}")),
    }
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
    command: &str,
) -> Result<String, CliError> {
    if let Some(value) = inline {
        if !value.is_empty()
            || matches!(
                command,
                "kill" | "create" | "run" | "events" | "export" | "update"
            )
            || (command == "logs" && name == "tail")
        {
            return Ok(value.to_string());
        }
    } else {
        *index += 1;
        if let Some(value) = args.get(*index) {
            return Ok(value.clone());
        }
    }
    let message = format!("option {name} requires a value");
    Err(if command == "kill" {
        CliError::command(message)
    } else {
        CliError::option(message)
    })
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
        "logs" if args.len() != 1 => Err("logs requires exactly one container".into()),
        "cp" if args.len() != 2 => Err("cp requires exactly two paths".into()),
        "attach" if args.len() != 1 || args[0].is_empty() => {
            Err("attach requires exactly one container".into())
        }
        "exec" if args.len() < 2 || args[0].is_empty() => {
            Err("exec requires a container and command".into())
        }
        "create" | "run" if args.is_empty() => {
            Err(format!("{command} requires an image or rootfs and command"))
        }
        "top" if args.is_empty() => Err("top requires a container".into()),
        "wait" if args.is_empty() || args.len() >= 1000 => {
            Err("wait requires between 1 and 999 containers".into())
        }
        "restart" if args.is_empty() || args.len() >= 1000 => {
            Err("restart requires between 1 and 999 containers".into())
        }
        "start" if args.is_empty() || args.len() >= 1000 => {
            Err("start requires between 1 and 999 containers".into())
        }
        "start" => {
            let mut attach = false;
            for option in options.iter().filter(|option| option.name == "attach") {
                attach = parse_bool(option.value.as_deref(), "attach")?;
            }
            if attach && args.len() > 1 {
                return Err("cannot start and attach multiple containers at once".into());
            }
            Ok(())
        }
        "rename" if args.len() != 2 => {
            Err("rename requires an old name and a new name".to_string())
        }
        "search" if args.len() != 1 || args[0].is_empty() => {
            Err("search requires one search term".to_string())
        }
        "logout" if args.len() != 1 || args[0].is_empty() => {
            Err("logout requires one registry server".to_string())
        }
        "login" if args.len() != 1 || args[0].is_empty() => {
            Err("login requires one registry server".to_string())
        }
        "tag" if args.len() != 2 || args.iter().any(String::is_empty) => {
            Err("tag requires a source and target image".to_string())
        }
        "import" if args.len() != 2 || args.iter().any(String::is_empty) => {
            Err("import requires a file and an image name".to_string())
        }
        "pull" if args.len() != 1 || args[0].is_empty() => {
            Err("pull requires one image name".to_string())
        }
        "load" if !args.is_empty() => Err("load takes no positional arguments".to_string()),
        "load" if !options.iter().any(|option| option.name == "input") => {
            Err("load requires -i or --input".to_string())
        }
        "rmi" if args.is_empty() => Err("rmi requires at least one image name".to_string()),
        "rmi" if args.len() >= 1000 => Err("too many images to remove".to_string()),
        "rm" if args.is_empty() => Err("rm requires at least one container name".to_string()),
        "rm" if args.len() >= 1000 => Err("too many containers to remove".to_string()),
        "stop" if args.is_empty() => Err("stop requires at least one container name".to_string()),
        "stop" if args.len() >= 1000 => Err("too many containers to stop".to_string()),
        "pause" if args.is_empty() => Err("pause requires at least one container name".to_string()),
        "pause" if args.len() >= 1000 => Err("too many containers to pause".to_string()),
        "unpause" if args.is_empty() => {
            Err("unpause requires at least one container name".to_string())
        }
        "unpause" if args.len() >= 1000 => Err("too many containers to unpause".to_string()),
        "ps" | "version" | "info" | "network ls" | "volume ls" | "volume prune"
            if !args.is_empty() =>
        {
            Err(format!("{command} takes no positional arguments"))
        }
        _ => Ok(()),
    }
}

pub fn parse(args: &[String], env_host: Option<&str>) -> Result<Action, CliError> {
    parse_inner(args, env_host).map_err(|mut error| {
        if args.first().is_some_and(|s| s == "volume")
            && args.get(1).is_some_and(|s| s == "ls")
            && !error.message.starts_with("unknown option:")
        {
            error.exit_code = 1;
        }
        error
    })
}

fn parse_inner(args: &[String], env_host: Option<&str>) -> Result<Action, CliError> {
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
                Some(option_value(args, &mut index, inline, name, command.name)?)
            } else if accepts_bool_value(command.name, spec.long) {
                parse_bool(inline, spec.long)?;
                inline.map(str::to_string)
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
                // Upstream kill/restart options accept -s=TERM/-t=1 or a separate value.
                // Without '=', remaining letters are parsed as more short options.
                if matches!(
                    command.name,
                    "kill" | "restart" | "create" | "run" | "exec" | "update"
                ) && spec.takes_value
                {
                    let inline = letters.as_str().strip_prefix('=');
                    let value = option_value(args, &mut index, inline, spec.long, command.name)?;
                    save_option(&mut host, &mut options, spec, Some(value));
                    if inline.is_some() {
                        break;
                    }
                    continue;
                }
                if accepts_bool_value(command.name, spec.long) {
                    if let Some(value) = letters.as_str().strip_prefix('=') {
                        parse_bool(Some(value), spec.long)?;
                        save_option(&mut host, &mut options, spec, Some(value.to_string()));
                        break;
                    }
                }
                let value = if spec.takes_value {
                    let rest = letters.as_str();
                    let inline = if let Some(value) = rest.strip_prefix('=') {
                        Some(value)
                    } else if rest.is_empty() {
                        None
                    } else {
                        Some(rest)
                    };
                    Some(option_value(
                        args,
                        &mut index,
                        inline,
                        spec.long,
                        command.name,
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

    if command.name == "update" {
        if positionals.is_empty() {
            return Err(CliError::option(
                "Update requires at least 1 container name".into(),
            ));
        }
        // Native checks the original argc, including the program name.
        if args.len() <= 2 {
            return Err(CliError::command(
                "You must provide one or more update flags when using this command".into(),
            ));
        }
        if positionals.len() >= 1000 {
            return Err(CliError::command("Too many containers to update".into()));
        }
    }

    check_host(&host).map_err(|message| {
        if command.name == "kill" {
            CliError::command(message)
        } else {
            CliError::option(message)
        }
    })?;
    check_args(command.name, &options, &positionals).map_err(|message| {
        if matches!(
            command.name,
            "create" | "run" | "start" | "stop" | "restart" | "pause" | "unpause" | "wait" | "top"
        ) {
            CliError::option(message)
        } else {
            CliError::command(message)
        }
    })?;

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
        assert_eq!(parse(&words("search"), None).unwrap_err().exit_code, 1);
        assert_eq!(
            parse(&words("search one two"), None).unwrap_err().exit_code,
            1
        );
        assert!(parse(
            &words("search --limit 10 -f stars=3 --no-trunc alpine"),
            None
        )
        .is_ok());
        assert_eq!(parse(&words("logout"), None).unwrap_err().exit_code, 1);
        assert_eq!(
            parse(&words("logout one two"), None).unwrap_err().exit_code,
            1
        );
        assert!(parse(&words("logout registry.test"), None).is_ok());
        assert_eq!(parse(&words("login"), None).unwrap_err().exit_code, 1);
        assert_eq!(
            parse(&words("login one two"), None).unwrap_err().exit_code,
            1
        );
        assert!(parse(&words("login -u user --password-stdin registry.test"), None).is_ok());
        assert_eq!(parse(&words("tag"), None).unwrap_err().exit_code, 1);
        assert_eq!(parse(&words("tag source"), None).unwrap_err().exit_code, 1);
        assert_eq!(
            parse(&words("tag source target extra"), None)
                .unwrap_err()
                .exit_code,
            1
        );
        assert!(parse(&words("tag source:old target:new"), None).is_ok());
        assert_eq!(parse(&words("import"), None).unwrap_err().exit_code, 1);
        assert_eq!(
            parse(&words("import image.tar"), None)
                .unwrap_err()
                .exit_code,
            1
        );
        assert_eq!(
            parse(&words("import a.tar image extra"), None)
                .unwrap_err()
                .exit_code,
            1
        );
        assert!(parse(&words("import image.tar example:test"), None).is_ok());
        assert_eq!(parse(&words("pull"), None).unwrap_err().exit_code, 1);
        assert_eq!(parse(&words("load"), None).unwrap_err().exit_code, 1);
        assert_eq!(parse(&words("rmi"), None).unwrap_err().exit_code, 1);
        assert!(parse(&words("rmi -f image:tag"), None).is_ok());
        let mut too_many = vec!["rmi".to_string()];
        too_many.extend(std::iter::repeat_n("image".to_string(), 1000));
        assert_eq!(parse(&too_many, None).unwrap_err().exit_code, 1);
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
