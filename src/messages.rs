//! MIDI message definitions and conversions.

use num_enum::TryFromPrimitive;

const STATUS_MASK: u8 = 0xF0;
const CHANNEL_MASK: u8 = 0x0F;

/// Representation of the MIDI message.
#[derive(Debug, Clone)]
pub struct MidiMessage {
    /// Message data.
    pub data: Vec<u8>,
}

impl MidiMessage {
    /// Returns the message status.
    pub fn status(&self) -> Status {
        let status_byte = self.data[0];
        let status = if status_byte >= 0xF0 {
            status_byte
        } else {
            status_byte & STATUS_MASK
        };

        match Status::try_from(status) {
            Ok(status) => status,
            Err(_) => Status::Error,
        }
    }

    /// Returns the message channel (0-based) or `None` for system messages.
    pub fn channel(&self) -> Option<u8> {
        let status_byte = self.data[0];
        if status_byte >= 0xF0 {
            None
        } else {
            Some(status_byte & CHANNEL_MASK)
        }
    }

    /// Returns a specific message data byte.
    pub fn data(&self, index: usize) -> u8 {
        self.data[index]
    }

    /// Returns the message data as 14-bit value.
    pub fn data_as_u16(&self) -> u16 {
        self.data[1] as u16 | ((self.data[2] as u16) << 7)
    }

    /// Creates a new message from an array
    pub fn from_array(data: &[u8]) -> MidiMessage {
        MidiMessage {
            data: Vec::from(data),
        }
    }

    /// Creates a new message from a `Vec`.
    pub fn from_vec(data: Vec<u8>) -> MidiMessage {
        MidiMessage { data }
    }

    /// Returns the note name for *Note Off*, *Note On* and *Poly Key Pressure* messages.
    ///
    /// Note no 60 is referred as C3.
    pub fn note_name(&self) -> Option<String> {
        match self.status() {
            Status::NoteOff | Status::NoteOn | Status::PolyKeyPressure => {
                let octave = self.data(1) as i32 / 12 - 2;
                let key = (self.data(1) % 12) as usize;
                let names = [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ];
                Some(format!("{}{}", String::from(names[key]), octave))
            }
            _ => None,
        }
    }

    /// Returns the name for most common control change messages.
    ///
    /// An empty string is returned for other controller numbers
    pub fn cc_name(&self) -> Option<String> {
        match self.status() {
            Status::ControlChange => {
                let name = match self.data(1) {
                    0 => "Bank Select MSB",
                    1 => "Mod Wheel",
                    2 => "Breath Control",
                    4 => "Foot Pedal",
                    5 => "Portamento Time",
                    6 => "Data Entry",
                    7 => "Volume",
                    8 => "Balance",
                    10 => "Pan",
                    11 => "Expression",
                    32 => "Bank Select LSB",
                    64 => "Sustain Pedal",
                    65 => "Portamento",
                    71 => "Timbre",
                    72 => "Release Time",
                    73 => "Attack Time",
                    74 => "Brightness",
                    91 => "Reverb Level",
                    93 => "Chorus Level",
                    120 => "All Sound Off",
                    121 => "All Controllers Off",
                    122 => "Local Control",
                    123 => "All Notes Off",
                    124 => "Omni Mode Off",
                    125 => "Omni Mode On",
                    126 => "Mono Mode",
                    127 => "Poly Mode",
                    _ => "",
                };
                Some(String::from(name))
            }
            _ => None,
        }
    }
}

/// Message status.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum Status {
    // Channel messages
    NoteOff = 0x80,
    NoteOn = 0x90,
    PolyKeyPressure = 0xA0,
    ControlChange = 0xB0,
    ProgramChange = 0xC0,
    ChannelPressure = 0xD0,
    PitchBend = 0xE0,

    // System common messages
    MtcQuarterFrame = 0xF1,
    SongPositionPointer = 0xF2,
    SongSelect = 0xF3,
    TuneRequest = 0xF6,
    EndOfExclusive = 0xF7,

    // System realtime messages
    TimingClock = 0xF8,
    Start = 0xFA,
    Continue = 0xFB,
    Stop = 0xFC,
    ActiveSensing = 0xFE,
    SystemReset = 0xFF,

    // System exclusive messages
    SystemExclusive = 0xF0,

    // Error
    Error = 0x00,
}

impl core::fmt::Display for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Status::NoteOff => "Note Off",
                Status::NoteOn => "Note On",
                Status::PolyKeyPressure => "Poly Key Pressure",
                Status::ControlChange => "Control Change",
                Status::ProgramChange => "Program Change",
                Status::ChannelPressure => "Channel Pressure",
                Status::PitchBend => "Pitch Bend",
                Status::SystemExclusive => "System Exclusive",
                Status::MtcQuarterFrame => "MTC Quarter Frame",
                Status::SongPositionPointer => "Song Position Pointer",
                Status::SongSelect => "Song Select",
                Status::TuneRequest => "Tune Request",
                Status::EndOfExclusive => "End of Exclusive",
                Status::TimingClock => "Timing Clock",
                Status::Start => "Start",
                Status::Continue => "Continue",
                Status::Stop => "Stop",
                Status::ActiveSensing => "Active Sensing",
                Status::SystemReset => "System Reset",
                Status::Error => "Error or unknown",
            }
        )
    }
}
