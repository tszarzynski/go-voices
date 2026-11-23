//! Sound categories, with the JUNO-DS codes `GO-sounds.md` uses.

use facet::Facet;
use serde::{Deserialize, Serialize};

/// What kind of sound a patch is. In the JUNO-DS's order.
#[derive(
    Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[repr(C)]
pub enum Category {
    /// Drum kits (`DRM`).
    Drums,
    /// Acoustic piano (`PNO`).
    Piano,
    /// Electric piano (`EP`).
    ElectricPiano,
    /// Keyboards (`KEY`).
    Keyboard,
    /// Bells (`BEL`).
    Bell,
    /// Mallets (`MLT`).
    Mallet,
    /// Organ (`ORG`).
    Organ,
    /// Accordion (`ACD`).
    Accordion,
    /// Harmonica (`HRM`).
    Harmonica,
    /// Acoustic guitar (`AGT`).
    AcousticGuitar,
    /// Electric guitar (`EGT`).
    ElectricGuitar,
    /// Distortion guitar (`DGT`).
    DistortionGuitar,
    /// Bass (`BS`).
    Bass,
    /// Synth bass (`SBS`).
    SynthBass,
    /// Strings (`STR`).
    Strings,
    /// Orchestra (`ORC`).
    Orchestra,
    /// Hits and stabs (`HIT`).
    Hit,
    /// Wind (`WND`).
    Wind,
    /// Flute (`FLT`).
    Flute,
    /// Brass (`BRS`).
    Brass,
    /// Synth brass (`SBR`).
    SynthBrass,
    /// Sax (`SAX`).
    Sax,
    /// Hard lead (`HLD`).
    HardLead,
    /// Soft lead (`SLD`).
    SoftLead,
    /// Techno synth (`TEK`).
    Techno,
    /// Pulsating (`PLS`).
    Pulsating,
    /// Synth FX (`FX`).
    SynthFx,
    /// Other synth (`SYN`).
    Synth,
    /// Bright pad (`BPD`).
    BrightPad,
    /// Soft pad (`SPD`).
    SoftPad,
    /// Voices (`VOX`).
    Vox,
    /// Plucked (`PLK`).
    Plucked,
    /// Ethnic (`ETH`).
    Ethnic,
    /// Fretted (`FRT`).
    Fretted,
    /// Percussion (`PRC`).
    Percussion,
    /// Sound FX (`SFX`).
    SoundFx,
    /// Beats and grooves (`BTS`).
    Beats,
    /// Combination (`CMB`).
    Combination,
    /// Sampled (`SMP`).
    Sample,
}

impl Category {
    /// Every category, in the JUNO-DS's order.
    pub const ALL: [Self; 39] = [
        Self::Drums,
        Self::Piano,
        Self::ElectricPiano,
        Self::Keyboard,
        Self::Bell,
        Self::Mallet,
        Self::Organ,
        Self::Accordion,
        Self::Harmonica,
        Self::AcousticGuitar,
        Self::ElectricGuitar,
        Self::DistortionGuitar,
        Self::Bass,
        Self::SynthBass,
        Self::Strings,
        Self::Orchestra,
        Self::Hit,
        Self::Wind,
        Self::Flute,
        Self::Brass,
        Self::SynthBrass,
        Self::Sax,
        Self::HardLead,
        Self::SoftLead,
        Self::Techno,
        Self::Pulsating,
        Self::SynthFx,
        Self::Synth,
        Self::BrightPad,
        Self::SoftPad,
        Self::Vox,
        Self::Plucked,
        Self::Ethnic,
        Self::Fretted,
        Self::Percussion,
        Self::SoundFx,
        Self::Beats,
        Self::Combination,
        Self::Sample,
    ];

    /// The JUNO-DS's code, e.g. `PNO`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Drums => "DRM",
            Self::Piano => "PNO",
            Self::ElectricPiano => "EP",
            Self::Keyboard => "KEY",
            Self::Bell => "BEL",
            Self::Mallet => "MLT",
            Self::Organ => "ORG",
            Self::Accordion => "ACD",
            Self::Harmonica => "HRM",
            Self::AcousticGuitar => "AGT",
            Self::ElectricGuitar => "EGT",
            Self::DistortionGuitar => "DGT",
            Self::Bass => "BS",
            Self::SynthBass => "SBS",
            Self::Strings => "STR",
            Self::Orchestra => "ORC",
            Self::Hit => "HIT",
            Self::Wind => "WND",
            Self::Flute => "FLT",
            Self::Brass => "BRS",
            Self::SynthBrass => "SBR",
            Self::Sax => "SAX",
            Self::HardLead => "HLD",
            Self::SoftLead => "SLD",
            Self::Techno => "TEK",
            Self::Pulsating => "PLS",
            Self::SynthFx => "FX",
            Self::Synth => "SYN",
            Self::BrightPad => "BPD",
            Self::SoftPad => "SPD",
            Self::Vox => "VOX",
            Self::Plucked => "PLK",
            Self::Ethnic => "ETH",
            Self::Fretted => "FRT",
            Self::Percussion => "PRC",
            Self::SoundFx => "SFX",
            Self::Beats => "BTS",
            Self::Combination => "CMB",
            Self::Sample => "SMP",
        }
    }

    /// What to call it, e.g. `Acoustic piano`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Drums => "Drum kits",
            Self::Piano => "Acoustic piano",
            Self::ElectricPiano => "Electric piano",
            Self::Keyboard => "Keyboards",
            Self::Bell => "Bells",
            Self::Mallet => "Mallets",
            Self::Organ => "Organ",
            Self::Accordion => "Accordion",
            Self::Harmonica => "Harmonica",
            Self::AcousticGuitar => "Acoustic guitar",
            Self::ElectricGuitar => "Electric guitar",
            Self::DistortionGuitar => "Distortion guitar",
            Self::Bass => "Bass",
            Self::SynthBass => "Synth bass",
            Self::Strings => "Strings",
            Self::Orchestra => "Orchestra",
            Self::Hit => "Hits and stabs",
            Self::Wind => "Wind",
            Self::Flute => "Flute",
            Self::Brass => "Brass",
            Self::SynthBrass => "Synth brass",
            Self::Sax => "Sax",
            Self::HardLead => "Hard lead",
            Self::SoftLead => "Soft lead",
            Self::Techno => "Techno synth",
            Self::Pulsating => "Pulsating",
            Self::SynthFx => "Synth FX",
            Self::Synth => "Other synth",
            Self::BrightPad => "Bright pad",
            Self::SoftPad => "Soft pad",
            Self::Vox => "Voices",
            Self::Plucked => "Plucked",
            Self::Ethnic => "Ethnic",
            Self::Fretted => "Fretted",
            Self::Percussion => "Percussion",
            Self::SoundFx => "Sound FX",
            Self::Beats => "Beats and grooves",
            Self::Combination => "Combination",
            Self::Sample => "Sampled",
        }
    }

    /// The category with this code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.code() == code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for category in Category::ALL {
            assert_eq!(Category::from_code(category.code()), Some(category));
        }
        assert_eq!(Category::from_code("pno"), None);
    }
}
