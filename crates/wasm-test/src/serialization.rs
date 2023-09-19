use clarity::vm::{
    types::{
        BuffData, CallableData, CharType, OptionalData, PrincipalData, QualifiedContractIdentifier,
        ResponseData, SequenceData, StandardPrincipalData, TraitIdentifier, Value,
    },
    ContractName,
};
use num::FromPrimitive;
use num_derive::{FromPrimitive, ToPrimitive};

use crate::Ptr;

pub const HEADER_LEN: i32 = 3;

#[derive(Debug, Clone, Copy)]
pub enum SerializationError {
    IndexOutOfRange,
    FailedToConvertBytesToAscii,
    FailedToConvertBytesToUtf8,
    InvalidTypeIndicator(u8),
    LengthIndicatorDoesNotMatchBufferLength,
    FailedToDeserializeLengthIndicator,
    InvalidBufferLength { expected: u16, received: u16 },
    FailedToDeserializeList,
    FailedToDeserializeListLength,
    AttemtToDeserializeZeroLengthBuffer,
    FailedToDeserializeContractName,
    FailedToDeserializeTraitName,
    FailedToDeserializePtr,
    InvalidPtrLength,
    TypeNotAllowed { received: TypeIndicator },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum TypeIndicator {
    UInt = 1,
    Int = 2,
    Bool = 3,
    Optional = 4,
    Response = 5,
    AsciiString = 6,
    Utf8String = 7,
    Buffer = 8,
    List = 9,
    StandardPrincipal = 10,
    ContractPrincipal = 11,
    CallableContract = 12,
    Tuple = 13,
}

impl TypeIndicator {
    /// Returns whether or not this type is an integer type.
    pub fn is_integer(&self) -> bool {
        self == &TypeIndicator::Int || self == &TypeIndicator::UInt
    }
}

/// Gets the type indicator value for the provided `Value`. This indicator is used to
/// prefix serialized values so that the type can be known during deserialization, especially
/// in the cases where multiple possible types are allowed in a `TypeSignature`.
#[inline]
fn get_type_indicator_for_clarity_value(value: &Value) -> u8 {
    match value {
        Value::UInt(_) => 1,
        Value::Int(_) => 2,
        Value::Bool(_) => 3,
        Value::Optional(_) => 4,
        Value::Response(_) => 5,
        Value::Sequence(SequenceData::String(CharType::ASCII(_))) => 6,
        Value::Sequence(SequenceData::String(CharType::UTF8(_))) => 7,
        Value::Sequence(SequenceData::Buffer(_)) => 8,
        Value::Sequence(SequenceData::List(_)) => 9,
        Value::Principal(PrincipalData::Standard(_)) => 10,
        Value::Principal(PrincipalData::Contract(_)) => 11,
        Value::CallableContract(_) => 12,
        Value::Tuple(_) => 13,
    }
}

/// Converts a serialized type indicator to a `TypeIndicator` enum variant. If the
/// byte provided does not map to a valid `TypeIndicator`, a `SerializationError` will
/// be returned.
#[inline]
fn type_indicator_byte_to_type_indicator(
    indicator: u8,
) -> Result<TypeIndicator, SerializationError> {
    let ind = TypeIndicator::from_u8(indicator);
    match ind {
        Some(i) => Ok(i),
        None => Err(SerializationError::InvalidTypeIndicator(indicator))?,
    }
}

#[inline]
pub fn get_type_indicator_from_serialized_value(
    data: &[u8],
) -> Result<TypeIndicator, SerializationError> {
    type_indicator_byte_to_type_indicator(data[0])
}

/// Deserializes a clarity sequence value (buffer, ascii, utf8, list, etc.) to a list of
/// `Ptr`s. This is used to allow the efficient iteration over a list's raw bytes without
/// actually deserializing the values. Useful for functions such as `fold` where we are
/// only passing the pointers further to the function to fold over.
#[inline]
pub fn deserialize_clarity_seq_to_ptrs(buffer: &[u8]) -> Result<Vec<Ptr>, SerializationError> {
    let type_indicator = type_indicator_byte_to_type_indicator(buffer[0])?;

    // This method only supports sequence types.
    if ![
        TypeIndicator::Buffer,
        TypeIndicator::AsciiString,
        TypeIndicator::Utf8String,
        TypeIndicator::List,
    ]
    .contains(&type_indicator)
    {
        Err(SerializationError::TypeNotAllowed {
            received: type_indicator,
        })?;
    }

    // Extract the length of this serialized value (excluding header).
    let length_indicator_bytes: [u8; 2] = buffer[1..=2]
        .try_into()
        .map_err(|_| SerializationError::FailedToDeserializeLengthIndicator)?;
    let length_indicator = u16::from_le_bytes(length_indicator_bytes);

    // Create a slice that contains only the value bytes (excluding the header).
    let value = &buffer[3..];

    // Ensure that our value slice length matches the parsed value length indicator.
    let value_length = value.len() as u16;
    if value_length != length_indicator {
        Err(SerializationError::LengthIndicatorDoesNotMatchBufferLength)?
    }

    // Split to retrieve the list length (first two bytes of the buffer)
    let (list_len_bytes, value_bytes) = value.split_at(2);

    // Deserialize the list length
    let list_len = u16::from_le_bytes(
        list_len_bytes
            .try_into()
            .map_err(|_| SerializationError::FailedToDeserializeListLength)?,
    );

    let mut ptrs = Vec::<Ptr>::with_capacity(list_len as usize);
    let mut index = 0;

    for _i in 0..list_len {
        // Deserialize the length of the next item
        let value_len = u16::from_le_bytes(
            value_bytes[(index + 1)..=(index + 2)]
                .try_into()
                .map_err(|_| SerializationError::FailedToDeserializeLengthIndicator)?,
        ) as usize;

        ptrs.push(Ptr::new(index as i32, value_len as i32));
        index += value_len + 3;
    }

    Ok(ptrs)
}

/// Deserializes a Clarity `Value` from the provided buffer using the given
/// `TypeSignature`. More documentation regarding how values are serialized
/// can be found in the `pass_argument_to_wasm` function.
#[inline]
pub fn deserialize_clarity_value(buffer: &[u8]) -> Result<Value, SerializationError> {
    // We cannot deserialize empty buffers.
    if buffer.is_empty() {
        Err(SerializationError::AttemtToDeserializeZeroLengthBuffer)?;
    }

    // Convert the type indicator byte to a `TypeIndicator`.
    let type_indicator = type_indicator_byte_to_type_indicator(buffer[0])?;

    // Extract the length of this serialized value (excluding header).
    let length_indicator_bytes: [u8; 2] = buffer[1..=2]
        .try_into()
        .map_err(|_| SerializationError::FailedToDeserializeLengthIndicator)?;
    let length_indicator = u16::from_le_bytes(length_indicator_bytes);

    // Create a slice that contains only the value bytes (excluding the header).
    let value = &buffer[3..];

    // Ensure that our value slice length matches the parsed value length indicator.
    let value_length = value.len() as u16;
    if value_length != length_indicator {
        Err(SerializationError::LengthIndicatorDoesNotMatchBufferLength)?
    }

    // Deserialize....
    let val = match type_indicator {
        TypeIndicator::UInt => {
            if value_length != 16 {
                Err(SerializationError::InvalidBufferLength {
                    expected: 16,
                    received: value_length,
                })?;
            }

            let bytes: [u8; 16] = value[0..16]
                .try_into()
                .map_err(|_| SerializationError::IndexOutOfRange)?;

            Value::UInt(u128::from_le_bytes(bytes))
        }
        TypeIndicator::Int => {
            if value_length != 16 {
                Err(SerializationError::InvalidBufferLength {
                    expected: 16,
                    received: value_length,
                })?;
            }

            let bytes: [u8; 16] = value[0..16]
                .try_into()
                .map_err(|_| SerializationError::IndexOutOfRange)?;

            Value::Int(i128::from_le_bytes(bytes))
        }
        TypeIndicator::Bool => {
            debug_assert!(
                value_length == 1,
                "Expected buffer length to be 1 for bool, received {value_length}"
            );

            let val = value[0];

            debug_assert!(
                value[0] == 1 || value[0] == 0,
                "Expected boolean value to be 1 or 0, received {val}"
            );

            Value::Bool(val == 1)
        }
        TypeIndicator::AsciiString => Value::string_ascii_from_bytes(value.to_vec())
            .map_err(|_| SerializationError::FailedToConvertBytesToAscii)?,
        TypeIndicator::Utf8String => Value::string_utf8_from_bytes(value.to_vec())
            .map_err(|_| SerializationError::FailedToConvertBytesToUtf8)?,
        TypeIndicator::Buffer => Value::Sequence(SequenceData::Buffer(BuffData {
            data: value.to_vec(),
        })),
        TypeIndicator::Response => {
            let inner_value = deserialize_clarity_value(&value[1..])?;

            // Read the first byte (indicator). 1/true = Ok, 0/false = Err.
            if value[0] == 1 {
                // If Ok, we will deserialize using the Ok `TypeSignature` (position 0 in the tuple).
                Value::Response(ResponseData {
                    committed: true,
                    data: Box::new(inner_value),
                })
            } else {
                // Otherwise if Err, we deserialize using the Err `TypeSignature` (position 1 in the tuple).
                Value::Response(ResponseData {
                    committed: false,
                    data: Box::new(inner_value),
                })
            }
        }
        TypeIndicator::Optional => {
            // Read the first byte (indicator). 1/true = Some, 0/false = None.
            if value[0] == 1 {
                // If Some, grab the remainder of the buffer and deserialize using the Option `TypeSignature`.
                // Note that there are no additional bytes if the value is None, so we only do this if we
                // have a Some indicator above.
                let val = deserialize_clarity_value(&value[1..])?;
                Value::Optional(OptionalData {
                    data: Some(Box::new(val)),
                })
            } else {
                // The indicator signals a None value, so we simply return None.
                Value::Optional(OptionalData { data: None })
            }
        }
        TypeIndicator::StandardPrincipal => {
            // Extract the standard principal data from the buffer.
            let standard_principal_data: [u8; 20] = value[1..=21]
                .try_into()
                .map_err(|_| SerializationError::IndexOutOfRange)?;

            let standard_principal = StandardPrincipalData(
                value[0],                // Version
                standard_principal_data, // Data
            );

            Value::Principal(PrincipalData::Standard(standard_principal))
        }
        TypeIndicator::ContractPrincipal => {
            // Extract the standard principal data from the buffer.
            let standard_principal_data: [u8; 20] = value[1..=21]
                .try_into()
                .map_err(|_| SerializationError::IndexOutOfRange)?;

            let standard_principal = StandardPrincipalData(
                value[0],                // Version
                standard_principal_data, // Data
            );

            // Parse out the contract name length
            let name_len_bytes: [u8; 2] = value[22..=23]
                .try_into()
                .map_err(|_| SerializationError::IndexOutOfRange)?;
            let name_len = u16::from_le_bytes(name_len_bytes) as usize;

            let mut name: &[u8] = &[];
            if name_len > 0 {
                name = &value[24..=(24 + name_len)];
            }

            // Convert the name to a string
            let name_str =
                std::str::from_utf8(name).map_err(|_| SerializationError::IndexOutOfRange)?;

            // Return the contract principal
            Value::Principal(PrincipalData::Contract(QualifiedContractIdentifier {
                issuer: standard_principal,
                name: name_str.into(),
            }))
        }
        TypeIndicator::List => {
            // Split to retrieve the list length (first two bytes of the buffer)
            let (list_len_bytes, value_bytes) = value.split_at(2);

            // Deserialize the list length
            let list_len = u16::from_le_bytes(
                list_len_bytes
                    .try_into()
                    .map_err(|_| SerializationError::FailedToDeserializeListLength)?,
            );

            let mut values = Vec::<Value>::with_capacity(list_len as usize);
            let mut index = 0;

            for _i in 0..list_len {
                // Deserialize the length of the next item
                let value_len = u16::from_le_bytes(
                    value_bytes[(index + 1)..=(index + 2)]
                        .try_into()
                        .map_err(|_| SerializationError::FailedToDeserializeLengthIndicator)?,
                ) as usize;

                let val_buffer = &value_bytes[index..=(index + value_len + 2)];
                let val = deserialize_clarity_value(val_buffer)?;
                values.push(val);
                index += value_len + 3;
            }

            Value::list_from(values).map_err(|_| SerializationError::FailedToDeserializeList)?
        }
        TypeIndicator::CallableContract => {
            // Extract the standard principal data from the buffer.
            let standard_principal_data: [u8; 20] = value[1..=21]
                .try_into()
                .map_err(|_| SerializationError::IndexOutOfRange)?;

            // Build the standard principal (contract identifier).
            let standard_principal = StandardPrincipalData(
                value[0],                // Version
                standard_principal_data, // Data
            );

            // Extract the contract name
            let ctr_name_len_bytes: [u8; 2] = value[22..=23]
                .try_into()
                .map_err(|_| SerializationError::FailedToDeserializeLengthIndicator)?;
            let ctr_name_len = u16::from_le_bytes(ctr_name_len_bytes);
            let ctr_name_str = std::str::from_utf8(&value[24..(24 + ctr_name_len as usize)])
                .map_err(|_| SerializationError::FailedToDeserializeContractName)?;

            // Build the `QualifiedContractIdentifier`.
            let contract_id = QualifiedContractIdentifier::new(
                standard_principal,
                ContractName::from(ctr_name_str),
            );

            let mut trait_id: Option<TraitIdentifier> = None;

            let (_, trait_bytes) = value.split_at(24 + ctr_name_len as usize);

            // If the trait identifier indicator is 1 then we also need to build
            // up the `TraitIdentifier`.
            if trait_bytes[0] == 1 {
                // Extract the trait principal data.
                let trait_principal_data: [u8; 20] = value[2..=22]
                    .try_into()
                    .map_err(|_| SerializationError::IndexOutOfRange)?;

                // Build the trait's standard principal.
                let trait_principal = StandardPrincipalData(trait_bytes[1], trait_principal_data);

                // Extract the trait name.
                let trait_name_len_bytes: [u8; 2] = trait_bytes[23..=24]
                    .try_into()
                    .map_err(|_| SerializationError::FailedToDeserializeLengthIndicator)?;
                let trait_name_len = u16::from_le_bytes(trait_name_len_bytes);
                let trait_name_str =
                    std::str::from_utf8(&trait_bytes[24..(24 + trait_name_len as usize)])
                        .map_err(|_| SerializationError::FailedToDeserializeTraitName)?;

                // Construct the trait identifier and attach it to the contract identifier.
                trait_id = Some(TraitIdentifier::new(
                    trait_principal,
                    contract_id.name.clone(),
                    trait_name_str.into(),
                ));
            }

            Value::CallableContract(CallableData {
                contract_identifier: contract_id,
                trait_identifier: trait_id,
            })
        }
        TypeIndicator::Tuple => {
            todo!("type not yet implemented")
        }
    };

    Ok(val)
}

/// Convert a Clarity 'Value' into a byte buffer. This is intended to be used
/// together with `pass_argument_to_wasm` for generating the buffer to be written
/// to WASM linear memory. More documentation regarding how values are serialized
/// can be found in the `pass_argument_to_wasm` function.
#[inline]
pub fn serialize_clarity_value(value: &Value) -> Result<Vec<u8>, SerializationError> {
    // Allocate a vector with a reasonably large capacity to avoid reallocations
    // in the majority of cases.
    let mut result = Vec::<u8>::with_capacity(256);
    let mut header = Vec::<u8>::with_capacity(result.len() + 3);

    // Insert the type marker.
    header.insert(0, get_type_indicator_for_clarity_value(value));

    match value {
        Value::UInt(n) => result.extend_from_slice(&n.to_le_bytes()),
        Value::Int(n) => {
            result.extend_from_slice(&n.to_le_bytes());
        }
        Value::Bool(b) => {
            result.insert(0, if *b { 1 } else { 0 });
        }
        Value::Optional(o) => {
            result.insert(0, if o.data.is_some() { 1 } else { 0 });
            if let Some(data) = &o.data {
                result.append(&mut serialize_clarity_value(data)?);
            }
        }
        Value::Response(r) => {
            result.insert(0, if r.committed { 1 } else { 0 });
            result.append(&mut serialize_clarity_value(&r.data)?);
        }
        Value::Sequence(SequenceData::String(char_type)) => match char_type {
            CharType::ASCII(s) => {
                result.extend_from_slice(&s.data);
            }
            CharType::UTF8(s) => {
                let mut data = s
                    .data
                    .iter()
                    .flat_map(|s| s.iter())
                    .map(|e| *e)
                    .collect::<Vec<u8>>();

                result.append(&mut data);
            }
        },
        Value::Sequence(SequenceData::Buffer(b)) => {
            result.extend_from_slice(&b.data);
        }
        Value::Sequence(SequenceData::List(l)) => {
            // Append the list length indicator
            result.extend_from_slice(&(l.data.len() as u16).to_le_bytes());

            // Append each list item
            for item in &l.data {
                let mut data = serialize_clarity_value(item)?;
                result.append(&mut data);
            }
        }
        Value::Principal(principal_type) => {
            match principal_type {
                PrincipalData::Standard(std) => {
                    // Write the version
                    result.insert(0, std.0);
                    // Write the principal data
                    result.extend_from_slice(&std.1);
                }
                PrincipalData::Contract(ctr) => {
                    // Write the version
                    result.insert(0, ctr.issuer.0);
                    // Write the principal data for the issuer
                    result.extend_from_slice(&ctr.issuer.1);

                    let name_bytes = ctr.name.as_bytes();
                    // Write a two-byte contract name length indicator.
                    result.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
                    // Write the name bytes.
                    result.extend_from_slice(name_bytes);
                }
            }
        }
        Value::CallableContract(ctr) => {
            // Write the contract identifier principal version.
            result.insert(0, ctr.contract_identifier.issuer.0);
            // Write the contract identifier principal data.
            result.extend_from_slice(&ctr.contract_identifier.issuer.1);

            // Handle the contract name
            let ctr_name_bytes = ctr.contract_identifier.name.as_bytes();
            // Write a two-byte contract name length indicator.
            result.extend_from_slice(&(ctr_name_bytes.len() as u16).to_le_bytes());
            // Write the contract name bytes.
            result.extend_from_slice(ctr_name_bytes);

            // If there is a trait identifier, append that after the contract principal.
            if let Some(trait_id) = &ctr.trait_identifier {
                // Write indicator for trait identifier presence.
                result.extend_from_slice(&[1]);
                // Write the trait identifier principal version.
                result.extend_from_slice(&[trait_id.contract_identifier.issuer.0]);
                // Write the trait identifier principal data.
                result.extend_from_slice(&trait_id.contract_identifier.issuer.1);

                // Handle the trait name
                let trait_name_bytes = trait_id.name.as_bytes();
                // Write a two-byte trait name length indicator.
                result.extend_from_slice(&(trait_name_bytes.len() as u16).to_le_bytes());
                // Write the trait name bytes.
                result.extend_from_slice(trait_name_bytes);
            } else {
                // Write indicator for no trait identifier presence.
                result.extend_from_slice(&[0]);
            }
        }
        Value::Tuple(tuple) => {
            for val in &tuple.data_map {
                let mut data = serialize_clarity_value(val.1)?;
                result.append(&mut data);
            }
        }
    }

    let len_bytes = (result.len() as u16).to_le_bytes();
    header.extend_from_slice(&len_bytes);

    header.append(&mut result);

    Ok(header)
}
