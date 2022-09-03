extern crate clap;
extern crate midir;

mod messages;

use clap::{value_t, App, Arg, SubCommand};
use messages::{MidiMessage, Status};
use midir::{ConnectError, MidiInput, MidiInputConnection};

/// Display format options
#[derive(Copy, Clone)]
enum DisplayFormat {
    Default,
    Raw,
    Min,
    MinHex,
}

/// Ignore flags for certain message types
#[derive(Copy, Clone)]
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

/// Filter to show only certain message types
#[derive(Copy, Clone)]
struct MessageFilter {
    channel: Option<u8>,
}

/// Application main function
fn main() {
    let app = App::new("MIDI monitor")
        .version("0.1")
        .author("Oliver Rockstedt <info@sourcebox.de>")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("ID")
                .help("Monitor single port"),
        )
        .arg(
            Arg::with_name("ignore")
                .short("i")
                .long("ignore")
                .value_name("TYPE")
                .multiple(true)
                .help("Ignore certain message types")
                .possible_values(&[
                    "note",
                    "polyat",
                    "cc",
                    "pc",
                    "at",
                    "pb",
                    "sysex",
                    "clock",
                    "sensing",
                    "realtime",
                    "transport",
                    "system",
                ]),
        )
        .arg(
            Arg::with_name("channel")
                .short("c")
                .long("channel")
                .value_name("CHANNEL")
                .help("Show only messages from specified channel"),
        )
        .arg(
            Arg::with_name("format")
                .short("f")
                .long("format")
                .value_name("FORMAT")
                .help("Display format")
                .possible_values(&["default", "raw", "min", "min-hex"]),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Suppress additional output"),
        )
        .subcommand(SubCommand::with_name("list").about("List available input ports"));

    let matches = app.get_matches();

    let result = if matches.is_present("list") {
        list_ports()
    } else {
        let format = match matches.value_of("format") {
            Some("raw") => DisplayFormat::Raw,
            Some("min") => DisplayFormat::Min,
            Some("min-hex") => DisplayFormat::MinHex,
            Some("default") | Some(&_) | None => DisplayFormat::Default,
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

        if let Some(ignores) = matches.values_of("ignore") {
            for i in ignores {
                match i {
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
            channel: if matches.is_present("channel") {
                Some(value_t!(matches.value_of("channel"), u8).unwrap_or_else(|e| e.exit()))
            } else {
                None
            },
        };

        let args = MonitorArgs {
            port: if matches.is_present("port") {
                Some(value_t!(matches.value_of("port"), u8).unwrap_or_else(|e| e.exit()))
            } else {
                None
            },
            format,
            ignore,
            filter,
            quiet: matches.is_present("quiet"),
        };
        monitor(args)
    };

    match result {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
}

/// List all available input ports
fn list_ports() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available input ports:");

    let midi_in = MidiInput::new("midimon input")?;

    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("  ({}) {}", i, midi_in.port_name(p)?);
    }

    Ok(())
}

// Monitor function arguments
struct MonitorArgs {
    port: Option<u8>,
    format: DisplayFormat,
    ignore: MessageIgnore,
    filter: MessageFilter,
    quiet: bool,
}

/// Monitor one or multiple input ports
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
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    Ok(())
}

/// Arguments for on_receive() callback function
struct ReceiveArgs {
    port_id: usize,
    format: DisplayFormat,
    ignore: MessageIgnore,
    filter: MessageFilter,
}

/// Receive callback function
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

/// Display message in default format
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

/// Display message in raw format
fn display_raw(port_id: usize, timestamp: u64, message: &[u8]) {
    println!(
        "  ({})  {:10.6}   {:?}",
        port_id,
        timestamp as f64 / 1e6,
        message
    );
}

/// Display message in min format
fn display_min(message: &[u8]) {
    let mut msg = Vec::<String>::new();

    for byte in message {
        msg.push(format!("{}", byte));
    }

    println!("{}", msg.join(", "));
}

/// Display message in min hex format
fn display_min_hex(message: &[u8]) {
    let mut msg = Vec::<String>::new();

    for byte in message {
        msg.push(format!("0x{:02X}", byte));
    }

    println!("{}", msg.join(", "));
}
