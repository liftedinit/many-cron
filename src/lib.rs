use many::{Identity, ManyError};

pub fn decode_identity(id: String) -> Result<Identity, ManyError> {
    let identity = if let Ok(data) = hex::decode(&id) {
        Identity::try_from(data.as_slice())?
    } else {
        Identity::try_from(id)?
    };
    Ok(identity)
}
