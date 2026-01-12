use std::{
    borrow::Cow,
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::Display,
    hash::Hash,
};

use super::{WireFormat, MAX_LABEL_LENGTH, MAX_NAME_LENGTH};

const POINTER_MASK: u8 = 0b1100_0000;
const POINTER_MASK_U16: u16 = 0b1100_0000_0000_0000;

// NOTE: there are no extend labels implemented today
// const EXTENDED_LABEL: u8 = 0b0100_0000;
// const EXTENDED_LABEL_U16: u16 = 0b0100_0000_0000_0000;

/// A Name represents a domain-name, which consists of character strings separated by dots.  
/// Each section of a name is called label  
/// ex: `google.com` consists of two labels `google` and `com`
///
/// A valid name contains only alphanumeric characters, hyphen (-), underscore (_) or dots (.) and must not exceed 255 characters.
/// Each label must not exceed 63 characters.
///
/// Microsoft implementation allows unicode characters in the name content.
/// To create a name with unicode characters, use [`Name::new_unchecked`] or
/// [`Name::new_with_labels`]
#[derive(Eq, Clone)]
pub struct Name<'a> {
    labels: Vec<Label<'a>>,
}

impl<'a> Name<'a> {
    /// Creates a new Name. Returns [`Result::<Name>::Ok`] if given `name` contents are valid.
    pub fn new(name: &'a str) -> crate::Result<Self> {
        let labels = LabelsIter::new(name.as_bytes())
            .map(Label::new)
            .collect::<Result<Vec<Label>, _>>()?;

        let name = Self { labels };

        if name.len() > MAX_NAME_LENGTH {
            Err(crate::SimpleDnsError::InvalidServiceName)
        } else {
            Ok(name)
        }
    }

    /// Create a new Name without checking for size limits or contents
    pub fn new_unchecked(name: &'a str) -> Self {
        let labels = LabelsIter::new(name.as_bytes())
            .map(Label::new_unchecked)
            .collect();

        Self { labels }
    }

    /// Creates a new Name with given labels
    ///
    /// Allows construction of labels with `.` in them.
    pub fn new_with_labels(labels: &[Label<'a>]) -> Self {
        Self {
            labels: labels.to_vec(),
        }
    }

    /// Verify if name ends with .local.
    pub fn is_link_local(&self) -> bool {
        match self.iter().last() {
            Some(label) => b"local".eq_ignore_ascii_case(&label.data),
            None => false,
        }
    }

    /// Returns an Iter of this Name Labels
    pub fn iter(&'a self) -> std::slice::Iter<'a, Label<'a>> {
        self.labels.iter()
    }

    /// Returns true if self is a subdomain of other
    pub fn is_subdomain_of(&self, other: &Name) -> bool {
        self.labels.len() > other.labels.len()
            && other
                .iter()
                .rev()
                .zip(self.iter().rev())
                .all(|(o, s)| *o == *s)
    }

    /// Returns the subdomain part of self, based on `domain`.
    /// If self is not a subdomain of `domain`, returns None
    ///
    /// Example:
    /// ```
    /// # use simple_dns::Name;
    /// let name = Name::new_unchecked("sub.domain.local");
    /// let domain = Name::new_unchecked("domain.local");
    ///
    /// assert!(domain.without(&name).is_none());
    ///
    /// let sub = name.without(&domain).unwrap();
    /// assert_eq!(sub.to_string(), "sub")
    /// ```
    pub fn without(&self, domain: &Name) -> Option<Name> {
        if self.is_subdomain_of(domain) {
            let labels = self.labels[..self.labels.len() - domain.labels.len()].to_vec();

            Some(Name { labels })
        } else {
            None
        }
    }

    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> Name<'b> {
        Name {
            labels: self.labels.into_iter().map(|l| l.into_owned()).collect(),
        }
    }

    /// Get the labels that compose this name
    pub fn get_labels(&'_ self) -> &'_ [Label<'_>] {
        &self.labels[..]
    }

    fn plain_append<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        for label in self.iter() {
            out.write_all(&[label.len() as u8])?;
            out.write_all(&label.data)?;
        }

        out.write_all(&[0])?;
        Ok(())
    }

    fn compress_append<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        for (i, label) in self.iter().enumerate() {
            match name_refs.entry(&self.labels[i..]) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    let p = *e.get() as u16;
                    out.write_all(&(p | POINTER_MASK_U16).to_be_bytes())?;

                    return Ok(());
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(out.stream_position()? as usize);
                    out.write_all(&[label.len() as u8])?;
                    out.write_all(&label.data)?;
                }
            }
        }

        out.write_all(&[0])?;
        Ok(())
    }

    /// Returns `true` if the name is valid.
    pub fn is_valid(&self) -> bool {
        self.labels.iter().all(|label| label.is_valid())
    }

    /// Returns the bytes of each of the labels that compose this Name
    pub fn as_bytes(&self) -> impl Iterator<Item = &[u8]> {
        self.labels.iter().map(|label| label.as_ref())
    }
}

impl<'a> WireFormat<'a> for Name<'a> {
    const MINIMUM_LEN: usize = 1;

    fn parse_after_check(data: &'a [u8], position: &mut usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let mut following_compression_pointer = false;
        let mut labels = Vec::new();

        let mut pointer_position = *position;

        // avoid invalid data caused oom
        let mut name_size = 0usize;

        loop {
            if pointer_position >= data.len() {
                return Err(crate::SimpleDnsError::InsufficientData);
            }

            // domain name max size is 255
            if name_size >= MAX_NAME_LENGTH {
                return Err(crate::SimpleDnsError::InvalidDnsPacket);
            }

            match data[pointer_position] {
                0 => {
                    *position += 1;
                    break;
                }
                len if len & POINTER_MASK == POINTER_MASK => {
                    if !following_compression_pointer {
                        *position += 1;
                    }

                    following_compression_pointer = true;
                    if pointer_position + 2 > data.len() {
                        return Err(crate::SimpleDnsError::InsufficientData);
                    }

                    // avoid pointer forward (RFC 1035)
                    let pointer = (u16::from_be_bytes(
                        data[pointer_position..pointer_position + 2].try_into()?,
                    ) & !POINTER_MASK_U16) as usize;
                    if pointer >= pointer_position {
                        return Err(crate::SimpleDnsError::InvalidDnsPacket);
                    }
                    pointer_position = pointer;
                }
                len => {
                    name_size += 1 + len as usize;
                    if pointer_position + 1 + len as usize > data.len() {
                        return Err(crate::SimpleDnsError::InsufficientData);
                    }

                    if len as usize > MAX_LABEL_LENGTH {
                        return Err(crate::SimpleDnsError::InvalidServiceLabel);
                    }

                    // Parsing allow invalid characters in the label.
                    // However, the length of the label must be validated (above)
                    labels.push(Label::new_unchecked(
                        &data[pointer_position + 1..pointer_position + 1 + len as usize],
                    ));

                    if !following_compression_pointer {
                        *position += len as usize + 1;
                    }
                    pointer_position += len as usize + 1;
                }
            }
        }

        Ok(Self { labels })
    }

    fn write_to<T: std::io::Write>(&self, out: &mut T) -> crate::Result<()> {
        self.plain_append(out)
    }

    fn write_compressed_to<T: std::io::Write + std::io::Seek>(
        &'a self,
        out: &mut T,
        name_refs: &mut HashMap<&'a [Label<'a>], usize>,
    ) -> crate::Result<()> {
        self.compress_append(out, name_refs)
    }

    fn len(&self) -> usize {
        self.labels
            .iter()
            .map(|label| label.len() + 1)
            .sum::<usize>()
            + Self::MINIMUM_LEN
    }
}

impl<'a> TryFrom<&'a str> for Name<'a> {
    type Error = crate::SimpleDnsError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Name::new(value)
    }
}

impl<'a> From<&'a [Label<'a>]> for Name<'a> {
    fn from(labels: &'a [Label<'a>]) -> Self {
        Name::new_with_labels(labels)
    }
}

impl<'a, const N: usize> From<[Label<'a>; N]> for Name<'a> {
    fn from(labels: [Label<'a>; N]) -> Self {
        Name::new_with_labels(&labels)
    }
}

impl Display for Name<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, label) in self.iter().enumerate() {
            if i != 0 {
                f.write_str(".")?;
            }

            f.write_fmt(format_args!("{}", label))?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for Name<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Name")
            .field(&format!("{}", self))
            .field(&format!("{}", self.len()))
            .finish()
    }
}

impl PartialEq for Name<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.labels == other.labels
    }
}

impl Hash for Name<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.labels.hash(state);
    }
}

/// An iterator over the labels in a domain name
struct LabelsIter<'a> {
    bytes: &'a [u8],
    current: usize,
}

impl<'a> LabelsIter<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, current: 0 }
    }
}

impl<'a> Iterator for LabelsIter<'a> {
    type Item = Cow<'a, [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        for i in self.current..self.bytes.len() {
            if self.bytes[i] == b'.' {
                let current = std::mem::replace(&mut self.current, i + 1);
                if i - current == 0 {
                    continue;
                }
                return Some(self.bytes[current..i].into());
            }
        }

        if self.current < self.bytes.len() {
            let current = std::mem::replace(&mut self.current, self.bytes.len());
            Some(self.bytes[current..].into())
        } else {
            None
        }
    }
}

/// Represents a label in a domain name
///
/// A valid label is consists of A-Z, a-z, 0-9, and hyphen (-), and must be at most 63 characters
/// in length.
/// This library also considers valid any label starting with underscore (_), to be able to parse mDNS domain names.
///
/// Microsoft implementation allows unicode characters in the label content.
/// To create a label with unicode characters, use [`Label::new_unchecked`]
///
/// The `Display` implementation uses [`std::string::String::from_utf8_lossy`] to display the
/// label.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct Label<'a> {
    data: Cow<'a, [u8]>,
}

impl<'a> Label<'a> {
    /// Create a new [`Label`] if given data is valid and within the limits
    pub fn new<T: Into<Cow<'a, [u8]>>>(data: T) -> crate::Result<Self> {
        let label = Self::new_unchecked(data);
        if !label.is_valid() {
            return Err(crate::SimpleDnsError::InvalidServiceLabel);
        }

        Ok(label)
    }

    /// Create a new Label without checking for size limits or valid content.
    /// This function can be used to create labels with unicode characters
    pub fn new_unchecked<T: Into<Cow<'a, [u8]>>>(data: T) -> Self {
        Self { data: data.into() }
    }

    /// Returns the length of the label
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the label is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Transforms the inner data into its owned type
    pub fn into_owned<'b>(self) -> Label<'b> {
        Label {
            data: self.data.into_owned().into(),
        }
    }

    /// Returns `true` if the label is valid.
    pub fn is_valid(&self) -> bool {
        if self.data.is_empty() || self.data.len() > MAX_LABEL_LENGTH {
            return false;
        }

        if let Some(first) = self.data.first() {
            if !first.is_ascii_alphanumeric() && *first != b'_' {
                return false;
            }
        }

        if !self
            .data
            .iter()
            .skip(1)
            .all(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_')
        {
            return false;
        }

        if let Some(last) = self.data.last() {
            if !last.is_ascii_alphanumeric() {
                return false;
            }
        }

        true
    }
}

impl Display for Label<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = std::string::String::from_utf8_lossy(&self.data);
        f.write_str(&s)
    }
}

impl std::fmt::Debug for Label<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("data", &self.to_string())
            .finish()
    }
}

impl AsRef<[u8]> for Label<'_> {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::{collections::hash_map::DefaultHasher, hash::Hasher};

    use super::*;
    use crate::SimpleDnsError;

    #[test]
    fn construct_valid_names() {
        assert!(Name::new("some").is_ok());
        assert!(Name::new("some.local").is_ok());
        assert!(Name::new("some.local.").is_ok());
        assert!(Name::new("some-dash.local.").is_ok());
        assert!(Name::new("_sync_miss._tcp.local").is_ok());
        assert!(Name::new("1sync_miss._tcp.local").is_ok());

        assert_eq!(Name::new_unchecked("\u{1F600}.local.").labels.len(), 2);
    }

    #[test]
    fn label_validate() {
        assert!(Name::new("\u{1F600}.local.").is_err());
        assert!(Name::new("@.local.").is_err());
        assert!(Name::new("\\.local.").is_err());
    }

    #[test]
    fn is_link_local() {
        assert!(!Name::new("some.example.com").unwrap().is_link_local());
        assert!(Name::new("some.example.local.").unwrap().is_link_local());
    }

    #[test]
    fn parse_without_compression() {
        let data =
            b"\x00\x00\x00\x01F\x03ISI\x04ARPA\x00\x03FOO\x01F\x03ISI\x04ARPA\x00\x04ARPA\x00";
        let mut position = 3;
        let name = Name::parse(data, &mut position).unwrap();
        assert_eq!("F.ISI.ARPA", name.to_string());

        let name = Name::parse(data, &mut position).unwrap();
        assert_eq!("FOO.F.ISI.ARPA", name.to_string());
    }

    #[test]
    fn parse_with_compression() {
        let data = b"\x00\x00\x00\x01F\x03ISI\x04ARPA\x00\x03FOO\xc0\x03\x03BAR\xc0\x03\x07INVALID\xc0\x1b";
        let mut offset = 3usize;

        let name = Name::parse(data, &mut offset).unwrap();
        assert_eq!("F.ISI.ARPA", name.to_string());

        let name = Name::parse(data, &mut offset).unwrap();
        assert_eq!("FOO.F.ISI.ARPA", name.to_string());

        let name = Name::parse(data, &mut offset).unwrap();
        assert_eq!("BAR.F.ISI.ARPA", name.to_string());

        assert!(Name::parse(data, &mut offset).is_err());
    }

    #[test]
    fn test_write() {
        let mut bytes = Cursor::new(Vec::with_capacity(30));
        Name::new_unchecked("_srv._udp.local")
            .write_to(&mut bytes)
            .unwrap();

        assert_eq!(b"\x04_srv\x04_udp\x05local\x00", &bytes.get_ref()[..]);

        let mut bytes = Cursor::new(Vec::with_capacity(30));
        Name::new_unchecked("_srv._udp.local2.")
            .write_to(&mut bytes)
            .unwrap();

        assert_eq!(b"\x04_srv\x04_udp\x06local2\x00", &bytes.get_ref()[..]);
    }

    #[test]
    fn root_name_should_generate_no_labels() {
        assert_eq!(Name::new_unchecked("").labels.len(), 0);
        assert_eq!(Name::new_unchecked(".").labels.len(), 0);
    }

    #[test]
    fn dot_sequence_should_generate_no_labels() {
        assert_eq!(Name::new_unchecked(".....").labels.len(), 0);
        assert_eq!(Name::new_unchecked("example.....com").labels.len(), 2);
    }

    #[test]
    fn root_name_should_write_zero() {
        let mut bytes = Cursor::new(Vec::with_capacity(30));
        Name::new_unchecked(".").write_to(&mut bytes).unwrap();

        assert_eq!(b"\x00", &bytes.get_ref()[..]);
    }

    #[test]
    fn append_to_vec_with_compression() {
        let mut buf = Cursor::new(vec![0, 0, 0]);
        buf.set_position(3);

        let mut name_refs = HashMap::new();

        let f_isi_arpa = Name::new_unchecked("F.ISI.ARPA");
        f_isi_arpa
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add F.ISI.ARPA");
        let foo_f_isi_arpa = Name::new_unchecked("FOO.F.ISI.ARPA");
        foo_f_isi_arpa
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add FOO.F.ISI.ARPA");

        Name::new_unchecked("BAR.F.ISI.ARPA")
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add FOO.F.ISI.ARPA");

        let data = b"\x00\x00\x00\x01F\x03ISI\x04ARPA\x00\x03FOO\xc0\x03\x03BAR\xc0\x03";
        assert_eq!(data[..], buf.get_ref()[..]);
    }

    #[test]
    fn append_to_vec_with_compression_mult_names() {
        let mut buf = Cursor::new(vec![]);
        let mut name_refs = HashMap::new();

        let isi_arpa = Name::new_unchecked("ISI.ARPA");
        isi_arpa
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add ISI.ARPA");

        let f_isi_arpa = Name::new_unchecked("F.ISI.ARPA");
        f_isi_arpa
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add F.ISI.ARPA");
        let foo_f_isi_arpa = Name::new_unchecked("FOO.F.ISI.ARPA");
        foo_f_isi_arpa
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add F.ISI.ARPA");
        Name::new_unchecked("BAR.F.ISI.ARPA")
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add F.ISI.ARPA");

        let expected = b"\x03ISI\x04ARPA\x00\x01F\xc0\x00\x03FOO\xc0\x0a\x03BAR\xc0\x0a";
        assert_eq!(expected[..], buf.get_ref()[..]);

        let mut position = 0;
        let first = Name::parse(buf.get_ref(), &mut position).unwrap();
        assert_eq!("ISI.ARPA", first.to_string());
        let second = Name::parse(buf.get_ref(), &mut position).unwrap();
        assert_eq!("F.ISI.ARPA", second.to_string());
        let third = Name::parse(buf.get_ref(), &mut position).unwrap();
        assert_eq!("FOO.F.ISI.ARPA", third.to_string());
        let fourth = Name::parse(buf.get_ref(), &mut position).unwrap();
        assert_eq!("BAR.F.ISI.ARPA", fourth.to_string());
    }

    #[test]
    fn ensure_different_domains_are_not_compressed() {
        let mut buf = Cursor::new(vec![]);
        let mut name_refs = HashMap::new();

        let foo_bar_baz = Name::new_unchecked("FOO.BAR.BAZ");
        foo_bar_baz
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add FOO.BAR.BAZ");

        let foo_bar_buz = Name::new_unchecked("FOO.BAR.BUZ");
        foo_bar_buz
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add FOO.BAR.BUZ");

        Name::new_unchecked("FOO.BAR")
            .write_compressed_to(&mut buf, &mut name_refs)
            .expect("failed to add FOO.BAR");

        let expected = b"\x03FOO\x03BAR\x03BAZ\x00\x03FOO\x03BAR\x03BUZ\x00\x03FOO\x03BAR\x00";
        assert_eq!(expected[..], buf.get_ref()[..]);
    }

    #[test]
    fn eq_other_name() -> Result<(), SimpleDnsError> {
        assert_eq!(Name::new("example.com")?, Name::new("example.com")?);
        assert_ne!(Name::new("some.example.com")?, Name::new("example.com")?);
        assert_ne!(Name::new("example.co")?, Name::new("example.com")?);
        assert_ne!(Name::new("example.com.org")?, Name::new("example.com")?);

        let data = b"\x00\x00\x00\x01F\x03ISI\x04ARPA\x00\x03FOO\xc0\x03\x03BAR\xc0\x03";
        let mut position = 3;
        assert_eq!(Name::new("F.ISI.ARPA")?, Name::parse(data, &mut position)?);
        assert_eq!(
            Name::new("FOO.F.ISI.ARPA")?,
            Name::parse(data, &mut position)?
        );
        Ok(())
    }

    #[test]
    fn len() -> crate::Result<()> {
        let mut bytes = Cursor::new(Vec::new());
        let name_one = Name::new_unchecked("ex.com.");
        name_one.write_to(&mut bytes)?;

        assert_eq!(8, bytes.get_ref().len());
        assert_eq!(bytes.get_ref().len(), name_one.len());
        assert_eq!(8, Name::parse(bytes.get_ref(), &mut 0)?.len());

        let mut name_refs = HashMap::new();
        let mut bytes = Cursor::new(Vec::new());
        name_one.write_compressed_to(&mut bytes, &mut name_refs)?;
        name_one.write_compressed_to(&mut bytes, &mut name_refs)?;

        assert_eq!(10, bytes.get_ref().len());
        Ok(())
    }

    #[test]
    fn hash() -> crate::Result<()> {
        let data = b"\x00\x00\x00\x01F\x03ISI\x04ARPA\x00\x03FOO\xc0\x03\x03BAR\xc0\x03";

        assert_eq!(
            get_hash(&Name::new("F.ISI.ARPA")?),
            get_hash(&Name::parse(data, &mut 3)?)
        );

        assert_eq!(
            get_hash(&Name::new("FOO.F.ISI.ARPA")?),
            get_hash(&Name::parse(data, &mut 15)?)
        );

        Ok(())
    }

    fn get_hash(name: &Name) -> u64 {
        let mut hasher = DefaultHasher::default();
        name.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn is_subdomain_of() {
        assert!(Name::new_unchecked("sub.example.com")
            .is_subdomain_of(&Name::new_unchecked("example.com")));

        assert!(!Name::new_unchecked("example.com")
            .is_subdomain_of(&Name::new_unchecked("example.com")));

        assert!(Name::new_unchecked("foo.sub.example.com")
            .is_subdomain_of(&Name::new_unchecked("example.com")));

        assert!(!Name::new_unchecked("example.com")
            .is_subdomain_of(&Name::new_unchecked("example.xom")));

        assert!(!Name::new_unchecked("domain.com")
            .is_subdomain_of(&Name::new_unchecked("other.domain")));

        assert!(!Name::new_unchecked("domain.com")
            .is_subdomain_of(&Name::new_unchecked("domain.com.br")));
    }

    #[test]
    fn subtract_domain() {
        let domain = Name::new_unchecked("_srv3._tcp.local");
        assert_eq!(
            Name::new_unchecked("a._srv3._tcp.local")
                .without(&domain)
                .unwrap()
                .to_string(),
            "a"
        );

        assert!(Name::new_unchecked("unrelated").without(&domain).is_none(),);

        assert_eq!(
            Name::new_unchecked("some.longer.domain._srv3._tcp.local")
                .without(&domain)
                .unwrap()
                .to_string(),
            "some.longer.domain"
        );
    }

    #[test]
    fn display_invalid_label() {
        let input = b"invalid\xF0\x90\x80label";
        let label = Label::new_unchecked(input);

        assert_eq!(label.to_string(), "invalid�label");
    }
}
