use crate::empty::Empty;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone)]
pub struct Version {
    pub version: String,
    pub parts: Vec<u32>,
}

impl Version {
    pub fn new(version: &str) -> Self {
        Self {
            version: version.trim().to_string(),
            parts: version
                .split('.')
                .map(|s| s.trim().parse().unwrap_or(0))
                .collect(),
        }
    }

    pub fn compare(&self, other: &Self) -> i32 {
        match self.cmp(other) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn strict_eq(&self, other: &Self) -> bool {
        self.version == other.version
    }

    pub fn as_str(&self) -> &str {
        self.version.as_str()
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let min_len = std::cmp::min(self.parts.len(), other.parts.len());
        for i in 0..min_len {
            match self.parts[i].cmp(&other.parts[i]) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        Ordering::Equal
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.version)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new("0.0.0")
    }
}

impl Empty for Version {
    fn is_empty(&self) -> bool {
        self.version == "0.0.0"
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let version = String::deserialize(deserializer)?;
        Ok(Self::new(&version))
    }
}

impl<T> From<T> for Version
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        Version::new(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn test_version_cmp() {
        let v1 = Version::new("1.0.0");
        let v2 = Version::new("1.0");
        assert_eq!(v1.compare(&v2), 1);
    }
}
