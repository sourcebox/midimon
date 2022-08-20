# About

midimon is a simple cross-platform command line utility to show incoming
messages from attached MIDI devices.

It is written in Rust and uses the [midir](https://github.com/Boddlnagg/midir) crate
for the low-level parts.

# Requirements

## Build

- [Rust toolchain](https://www.rust-lang.org/)

Use `cargo build` to compile or `cargo run` to compile and run.

## Tested target platforms

- Linux Mint 18.3
- macOS 10.12 (Sierra)
- Windows 7

Other OS versions will most likely work, but were not tested explicitly.

# Basic usage

To monitor all messages from any input port, run the command without any options.

    ./midimon

Use *Ctrl-C* to stop.

# Getting help

The `h` or `--help` flag will give you an overview about all available options.

    ./midimon --help

# Options

## List available input ports

This subcommand shows a list of all available input ports and their numerical ids. The ids are used as abbreviation in the messages list and for the port filter option.

    ./midimon list

## Monitor a single port

The option `-p` or `--port` restricts monitoring to a single input port.

Example:

    ./midimon -p 3

This will show messages from port id 3 only. To find out which physical port refers to each numerical id, use the `list` subcommand.

## Ignoring messages

Use the option `-i` or `--ignore` to suppress certain message types.

Possible types:

    note        Note Off, Note On
    polyat      Polyphonic Key Pressure
    cc          Control Change
    pc          Program Change
    at          Channel Pressure (Aftertouch)
    pb          Pitch Bend
    sysex       System Exclusive
    clock       Timing Clock
    sensing     Active Sensing
    realtime    All realtime messages (Clock, Start, Stop, Continue, Active Sensing, Reset)
    transport   Start, Stop and Continue messages
    system      All system messages

Example:

    ./midimon -i clock sensing

This will ignore incoming clock and active sensing messages.

## Channel filter

Use the option `-c` or `--channel` to display only messages from a single channel.

Example:

    ./midimon -c 10

This will only show messages from MIDI channel 10.

*Note:* System messages are also displayed when using the channel filter. If this is not desired,
use the ignore option in addition.

## Suppressing informational output

The `-q` or `--quiet` option suppresses any informational output like the used ports info.
The main use case for this option is when capturing data by redirecting the output to a file.

## Display formats

The default display format is intended to be informational and therefore shows an interpreted
view of the incoming data.

Use the option `-f` or  `--format` to select an alternate display format.

Example:

    ./midimon -f raw

Shows the each message in an uninterpreted list-style format.

Example:

    ./midimon -f min

Monitor messages using a bare minimum display format.

Example:

    ./midimon -f min-hex

Same as the `-f min` option, but with hexadecimal output format.

It is recommended to use these options in combination with the `-p` option to restrict the monitoring to a single port.

# Tips

## Capturing data into a file

In case you want to capture some output from a device into a file, e.g. a SysEx dump,
use some combination of arguments and a shell redirect.

Example:

    ./midimon -f min-hex -p 1 -q > midi_capture.txt

*Note:* This will only work with shells like bash that support redirects.
