use serde::{Deserialize, Serialize};
use strum_macros::{EnumString, EnumVariantNames, FromRepr};

#[derive(
    Debug, Clone, Copy, PartialEq, Deserialize, Serialize, EnumString, EnumVariantNames, FromRepr,
)]
#[strum(serialize_all = "kebab_case")]
pub enum Format {
    Mp3V0,
    Mp3,
    Flac,
    Aac,
    OggVorbis,
    Alac,
    Wav,
    Aiff,
}
