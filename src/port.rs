//! Port newtype for type-safe port numbers.
//!
//! Provides a `Port` type that wraps `u16` with validation to ensure
//! port numbers are within the valid TCP/UDP range (1-65535).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A validated TCP/UDP port number (1-65535).
///
/// This newtype ensures that port numbers are always valid at construction time,
/// preventing invalid ports from propagating through the codebase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port(u16);

/// Error returned when attempting to create an invalid port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPortError {
    /// The invalid port value that was attempted.
    pub value: u16,
}

impl fmt::Display for InvalidPortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid port number: {} (must be 1-65535)", self.value)
    }
}

impl std::error::Error for InvalidPortError {}

impl Port {
    /// The minimum valid port number.
    pub const MIN: u16 = 1;

    /// The maximum valid port number.
    pub const MAX: u16 = 65535;

    /// Creates a new `Port` from a `u16` value.
    ///
    /// # Errors
    ///
    /// Returns `InvalidPortError` if the value is 0 (port 0 is reserved).
    pub fn new(value: u16) -> Result<Self, InvalidPortError> {
        if value == 0 {
            Err(InvalidPortError { value })
        } else {
            Ok(Port(value))
        }
    }

    /// Returns the port number as a `u16`.
    #[inline]
    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u16> for Port {
    type Error = InvalidPortError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Port::new(value)
    }
}

impl From<Port> for u16 {
    fn from(port: Port) -> Self {
        port.0
    }
}

impl FromStr for Port {
    type Err = PortParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u16 = s
            .parse()
            .map_err(|_| PortParseError::InvalidFormat(s.to_string()))?;
        Port::new(value).map_err(|e| PortParseError::InvalidValue(e.value))
    }
}

/// Error returned when parsing a port from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortParseError {
    /// The string could not be parsed as a number.
    InvalidFormat(String),
    /// The number was not a valid port (e.g., 0).
    InvalidValue(u16),
}

impl fmt::Display for PortParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PortParseError::InvalidFormat(s) => write!(f, "invalid port format: '{s}'"),
            PortParseError::InvalidValue(v) => {
                write!(f, "invalid port number: {v} (must be 1-65535)")
            }
        }
    }
}

impl std::error::Error for PortParseError {}

impl Serialize for Port {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Port {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Port::new(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ports() {
        assert!(Port::new(1).is_ok());
        assert!(Port::new(80).is_ok());
        assert!(Port::new(443).is_ok());
        assert!(Port::new(8080).is_ok());
        assert!(Port::new(65535).is_ok());
    }

    #[test]
    fn test_invalid_port_zero() {
        let result = Port::new(0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().value, 0);
    }

    #[test]
    fn test_as_u16() {
        let port = Port::new(8080).unwrap();
        assert_eq!(port.as_u16(), 8080);
    }

    #[test]
    fn test_display() {
        let port = Port::new(8080).unwrap();
        assert_eq!(format!("{port}"), "8080");
    }

    #[test]
    fn test_try_from() {
        let port: Result<Port, _> = 8080u16.try_into();
        assert!(port.is_ok());
        assert_eq!(port.unwrap().as_u16(), 8080);

        let invalid: Result<Port, _> = 0u16.try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_from_str() {
        assert_eq!("8080".parse::<Port>().unwrap().as_u16(), 8080);
        assert!("0".parse::<Port>().is_err());
        assert!("abc".parse::<Port>().is_err());
        assert!("".parse::<Port>().is_err());
    }

    #[test]
    fn test_ordering() {
        let p1 = Port::new(80).unwrap();
        let p2 = Port::new(443).unwrap();
        let p3 = Port::new(8080).unwrap();

        assert!(p1 < p2);
        assert!(p2 < p3);
    }

    #[test]
    fn test_serde_roundtrip() {
        let port = Port::new(8080).unwrap();
        let json = serde_json::to_string(&port).unwrap();
        assert_eq!(json, "8080");

        let deserialized: Port = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, port);
    }

    #[test]
    fn test_serde_invalid_port() {
        let result: Result<Port, _> = serde_json::from_str("0");
        assert!(result.is_err());
    }
}
