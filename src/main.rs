#![doc = include_str!("../README.md")]

pub mod messages;

use clap::{builder::PossibleValue, value_parser, Arg, ArgAction, Command};
use midir::{ConnectError, MidiInput, MidiInputConnection};

use messages::{MidiMessage, Status};

/// Display format options.
#[derive(Debug, Copy, Clone)]
enum DisplayFormat {
    Default,
    Raw,
    Min,
    MinHex,
}

/// Ignore flags for certain message types.
#[derive(Debug, Copy, Clone)]
struct MessageIgnore {
    note: bool,
    poly_pressure: bool,
    control_change: bool,
    program_change: bool,
    channel_pressure: bool,
    pitch_bend: bool,
    sysex: bool,
    mtc_frame: bool,
    song_pos_pointer: bool,
    song_select: bool,
    tune_request: bool,
    clock: bool,
    start: bool,
    continue_: bool,
    stop: bool,
    sensing: bool,
    reset: bool,
}

/// Filter to show only certain message types.
#[derive(Debug, Copy, Clone)]
struct MessageFilter {
    channel: Option<u8>,
}

/// Application entry point.
fn main() {
    let command = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("ID")
                .help("Monitor single port")
                .value_parser(value_parser!(u8)),
        )
        .arg(
            Arg::new("ignore")
                .short('i')
                .long("ignore")
                .value_name("TYPE")
                .num_args(1..)
                .help("Ignore certain message types")
                .value_parser([
                    PossibleValue::new("note"),
                    PossibleValue::new("polyat"),
                    PossibleValue::new("cc"),
                    PossibleValue::new("pc"),
                    PossibleValue::new("at"),
                    PossibleValue::new("pb"),
                    PossibleValue::new("sysex"),
                    PossibleValue::new("clock"),
                    PossibleValue::new("sensing"),
                    PossibleValue::new("realtime"),
                    PossibleValue::new("transport"),
                    PossibleValue::new("system"),
                ]),
        )
        .arg(
            Arg::new("channel")
                .short('c')
                .long("channel")
                .value_name("CHANNEL")
                .help("Show only messages from specified channel")
                .value_parser(value_parser!(u8)),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Display format")
                .default_value("default")
                .value_parser([
                    PossibleValue::new("default"),
                    PossibleValue::new("raw"),
                    PossibleValue::new("min"),
                    PossibleValue::new("min-hex"),
                ]),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(ArgAction::SetTrue)
                .help("Suppress additional output"),
        )
        .subcommand(Command::new("list").about("List available input ports"));

    let matches = command.get_matches();

    let result = match matches.subcommand() {
        Some(("list", _)) => list_ports(),
        _ => {
            let format = match matches
                .get_one::<String>("format")
                .expect("Format spec missing")
                .as_str()
            {
                "raw" => DisplayFormat::Raw,
                "min" => DisplayFormat::Min,
                "min-hex" => DisplayFormat::MinHex,
                _ => DisplayFormat::Default,
            };

            let mut ignore = MessageIgnore {
                note: false,
                poly_pressure: false,
                control_change: false,
                program_change: false,
                channel_pressure: false,
                pitch_bend: false,
                sysex: false,
                mtc_frame: false,
                song_pos_pointer: false,
                song_select: false,
                tune_request: false,
                clock: false,
                start: false,
                continue_: false,
                stop: false,
                sensing: false,
                reset: false,
            };

            if let Some(ignores) = matches.get_many::<String>("ignore") {
                for i in ignores {
                    match i.as_str() {
                        "note" => ignore.note = true,
                        "polyat" => ignore.poly_pressure = true,
                        "cc" => ignore.control_change = true,
                        "pc" => ignore.program_change = true,
                        "at" => ignore.channel_pressure = true,
                        "pb" => ignore.pitch_bend = true,
                        "sysex" => ignore.sysex = true,
                        "clock" => ignore.clock = true,
                        "sensing" => ignore.sensing = true,
                        "realtime" => {
                            ignore.clock = true;
                            ignore.start = true;
                            ignore.continue_ = true;
                            ignore.stop = true;
                            ignore.sensing = true;
                            ignore.reset = true;
                        }
                        "transport" => {
                            ignore.start = true;
                            ignore.continue_ = true;
                            ignore.stop = true;
                        }
                        "system" => {
                            ignore.sysex = true;
                            ignore.mtc_frame = true;
                            ignore.song_pos_pointer = true;
                            ignore.song_select = true;
                            ignore.tune_request = true;
                            ignore.clock = true;
                            ignore.start = true;
                            ignore.continue_ = true;
                            ignore.stop = true;
                            ignore.sensing = true;
                            ignore.reset = true;
                        }
                        &_ => (),
                    }
                }
            };

            let filter = MessageFilter {
                channel: if matches.contains_id("channel") {
                    Some(
                        matches
                            .get_one::<u8>("channel")
                            .expect("Channel argument missing.")
                            .to_owned(),
                    )
                } else {
                    None
                },
            };

            let args = MonitorArgs {
                port: if matches.contains_id("port") {
                    Some(
                        matches
                            .get_one::<u8>("port")
                            .expect("Port argument missing.")
                            .to_owned(),
                    )
                } else {
                    None
                },
                format,
                ignore,
                filter,
                quiet: matches.get_flag("quiet"),
            };
            monitor(args)
        }
    };

    match result {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

/// Lists all available input ports.
fn list_ports() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available input ports:");

    let midi_in = MidiInput::new("midimon input")?;

    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("  ({}) {}", i, midi_in.port_name(p)?);
    }

    Ok(())
}

// Monitor function arguments.
#[derive(Debug)]
struct MonitorArgs {
    port: Option<u8>,
    format: DisplayFormat,
    ignore: MessageIgnore,
    filter: MessageFilter,
    quiet: bool,
}

/// Monitors one or multiple input ports.
#[allow(unreachable_code)]
fn monitor(args: MonitorArgs) -> Result<(), Box<dyn std::error::Error>> {
    let midi_in = MidiInput::new("midimon input")?;

    type Connection = Result<MidiInputConnection<ReceiveArgs>, ConnectError<MidiInput>>;

    let mut connections = Vec::<Connection>::new();

    let show_info = !args.quiet;

    if show_info {
        println!("Active input ports:");
    }

    for (i, in_port) in midi_in.ports().iter().enumerate() {
        let midi_in = MidiInput::new("midimon input")?;
        let port_name = midi_in.port_name(in_port)?;
        let add_connection = if let Some(port_id) = args.port {
            port_id == i as u8
        } else {
            true
        };

        if add_connection {
            if show_info {
                println!("  ({}) {}", i, port_name);
            }

            let receive_args = ReceiveArgs {
                port_id: i,
                format: args.format,
                ignore: args.ignore,
                filter: args.filter,
            };
            connections.push(midi_in.connect(in_port, "input monitor", on_receive, receive_args));
        }
    }

    if show_info {
        let mut ignore_info: Vec<String> = Vec::new();

        if args.ignore.note {
            ignore_info.push("Note Off, Note On".to_string());
        }
        if args.ignore.poly_pressure {
            ignore_info.push("Poly Key Pressure".to_string());
        }
        if args.ignore.control_change {
            ignore_info.push("Control Change".to_string());
        }
        if args.ignore.program_change {
            ignore_info.push("Program Change".to_string());
        }
        if args.ignore.pitch_bend {
            ignore_info.push("Pitch Bend".to_string());
        }
        if args.ignore.sysex {
            ignore_info.push("Sysex".to_string());
        }
        if args.ignore.mtc_frame {
            ignore_info.push("MTC Quarter Frame".to_string());
        }
        if args.ignore.song_pos_pointer {
            ignore_info.push("Song Pos Pointer".to_string());
        }
        if args.ignore.song_select {
            ignore_info.push("Song Select".to_string());
        }
        if args.ignore.tune_request {
            ignore_info.push("Tune Request".to_string());
        }
        if args.ignore.clock {
            ignore_info.push("Clock".to_string());
        }
        if args.ignore.start {
            ignore_info.push("Start".to_string());
        }
        if args.ignore.continue_ {
            ignore_info.push("Continue".to_string());
        }
        if args.ignore.stop {
            ignore_info.push("Stop".to_string());
        }
        if args.ignore.sensing {
            ignore_info.push("Active Sensing".to_string());
        }
        if args.ignore.reset {
            ignore_info.push("Reset".to_string());
        }

        if !ignore_info.is_empty() {
            println!("Ignoring {}", ignore_info.join(", "));
        }

        if let Some(channel) = args.filter.channel {
            println!("Using channel filter {}", channel);
        }

        println!("Listening... Press Ctrl-C to exit.");
    }

    loop {
        std::thread::sleep(core::time::Duration::from_millis(10));
    }

    Ok(())
}

/// Arguments for the `on_receive()` callback function.
#[derive(Debug)]
struct ReceiveArgs {
    port_id: usize,
    format: DisplayFormat,
    ignore: MessageIgnore,
    filter: MessageFilter,
}

/// Receive callback function.
fn on_receive(timestamp: u64, message: &[u8], args: &mut ReceiveArgs) {
    let status = if message[0] >= 0xF0 {
        message[0]
    } else {
        message[0] & 0xF0
    };

    if args.ignore.note && (status == Status::NoteOff as u8 || status == Status::NoteOn as u8) {
        return;
    }

    if args.ignore.poly_pressure && (status == Status::PolyKeyPressure as u8) {
        return;
    }

    if args.ignore.control_change && (status == Status::ControlChange as u8) {
        return;
    }

    if args.ignore.program_change && (status == Status::ProgramChange as u8) {
        return;
    }

    if args.ignore.channel_pressure && (status == Status::ChannelPressure as u8) {
        return;
    }

    if args.ignore.pitch_bend && (status == Status::PitchBend as u8) {
        return;
    }

    if args.ignore.sysex && (status == Status::SystemExclusive as u8) {
        return;
    }

    if args.ignore.mtc_frame && (status == Status::MtcQuarterFrame as u8) {
        return;
    }

    if args.ignore.song_pos_pointer && (status == Status::SongPositionPointer as u8) {
        return;
    }

    if args.ignore.song_select && (status == Status::SongSelect as u8) {
        return;
    }

    if args.ignore.tune_request && (status == Status::TuneRequest as u8) {
        return;
    }

    if args.ignore.clock && (status == Status::TimingClock as u8) {
        return;
    }

    if args.ignore.start && (status == Status::Start as u8) {
        return;
    }

    if args.ignore.continue_ && (status == Status::Continue as u8) {
        return;
    }

    if args.ignore.stop && (status == Status::Stop as u8) {
        return;
    }

    if args.ignore.sensing && (status == Status::ActiveSensing as u8) {
        return;
    }

    if args.ignore.reset && (status == Status::SystemReset as u8) {
        return;
    }

    if let Some(channel) = args.filter.channel {
        if (message[0] <= Status::SystemExclusive as u8) && (message[0] & 0x0F != channel - 1) {
            return;
        }
    }

    match args.format {
        DisplayFormat::Default => display_default(args.port_id, timestamp, message),
        DisplayFormat::Raw => display_raw(args.port_id, timestamp, message),
        DisplayFormat::Min => display_min(message),
        DisplayFormat::MinHex => display_min_hex(message),
    }
}

/// Displays a message in default format.
fn display_default(port_id: usize, timestamp: u64, message: &[u8]) {
    let msg = MidiMessage::from_array(message);

    let status_text = format!("{}", msg.status());

    let data_text = match msg.status() {
        Status::NoteOff | Status::NoteOn => format!(
            "Ch:{:>2}  Note:{:>3}  Vel:{:>3}    {}",
            msg.channel().unwrap() + 1,
            msg.data(1),
            msg.data(2),
            msg.note_name().unwrap()
        ),
        Status::PolyKeyPressure => format!(
            "Ch:{:>2}  Note:{:>3}  Val:{:>3}    {}",
            msg.channel().unwrap() + 1,
            msg.data(1),
            msg.data(2),
            msg.note_name().unwrap()
        ),
        Status::ControlChange => format!(
            "Ch:{:>2}  No:  {:>3}  Val:{:>3}    {}",
            msg.channel().unwrap() + 1,
            msg.data(1),
            msg.data(2),
            msg.cc_name().unwrap()
        ),
        Status::ProgramChange | Status::ChannelPressure => format!(
            "Ch:{:>2}  Val:{:>3}",
            msg.channel().unwrap() + 1,
            msg.data(1),
        ),
        Status::PitchBend => format!(
            "Ch:{:>2}  Val:{:>5}",
            msg.channel().unwrap() + 1,
            msg.data_as_u16() as i16 - 0x2000,
        ),
        Status::MtcQuarterFrame | Status::SongSelect => format!("{:>3}", msg.data(1)),
        Status::SongPositionPointer => format!("{:>3}  {:>3}", msg.data(1), msg.data(2)),
        Status::TuneRequest
        | Status::TimingClock
        | Status::Start
        | Status::Continue
        | Status::Stop
        | Status::ActiveSensing
        | Status::SystemReset => String::new(),
        _ => format!("{:?}", msg.data),
    };

    println!(
        "  ({})  {:10.6}  {:21}  {}",
        port_id,
        timestamp as f64 / 1e6,
        status_text,
        data_text
    );
}

/// Displays a message in raw format.
fn display_raw(port_id: usize, timestamp: u64, message: &[u8]) {
    println!(
        "  ({})  {:10.6}   {:?}",
        port_id,
        timestamp as f64 / 1e6,
        message
    );
}

/// Displays a message in min format.
fn display_min(message: &[u8]) {
    let mut msg = Vec::<String>::new();

    for byte in message {
        msg.push(format!("{}", byte));
    }

    println!("{}", msg.join(", "));
}

/// Displays a message in min hex format.
fn display_min_hex(message: &[u8]) {
    let mut msg = Vec::<String>::new();

    for byte in message {
        msg.push(format!("0x{:02X}", byte));
    }

    println!("{}", msg.join(", "));
}
