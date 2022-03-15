//! Some utility functions that don't fit anywhere

use many::{Identity, ManyError};

/// Decode an Identity from a string
pub fn decode_identity(id: String) -> Result<Identity, ManyError> {
    let identity = if let Ok(data) = hex::decode(&id) {
        Identity::try_from(data.as_slice())?
    } else {
        Identity::try_from(id)?
    };
    Ok(identity)
}
