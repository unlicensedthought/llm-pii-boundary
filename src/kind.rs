//! What a placeholder stands for.

/// What a placeholder stands for. The tag is what appears in the placeholder: `[EMAIL_1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Email,
    Phone,
    Ssn,
    Card,
    Iban,
    Ip,
    Url,
    Address,
    Postcode,
    Date,
    Id,
    /// One of the caller's own identity terms (their name, employer, town).
    Term,
    Person,
    Org,
    Place,
}

impl Kind {
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Email => "EMAIL",
            Kind::Phone => "PHONE",
            Kind::Ssn => "SSN",
            Kind::Card => "CARD",
            Kind::Iban => "IBAN",
            Kind::Ip => "IP",
            Kind::Url => "URL",
            Kind::Address => "ADDRESS",
            Kind::Postcode => "POSTCODE",
            Kind::Date => "DATE",
            Kind::Id => "ID",
            Kind::Term => "TERM",
            Kind::Person => "PERSON",
            Kind::Org => "ORG",
            Kind::Place => "PLACE",
        }
    }

    /// The key in `meta.pii.kinds`.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Email => "email",
            Kind::Phone => "phone",
            Kind::Ssn => "ssn",
            Kind::Card => "card",
            Kind::Iban => "iban",
            Kind::Ip => "ip",
            Kind::Url => "url",
            Kind::Address => "address",
            Kind::Postcode => "postcode",
            Kind::Date => "date",
            Kind::Id => "id",
            Kind::Term => "term",
            Kind::Person => "person",
            Kind::Org => "org",
            Kind::Place => "place",
        }
    }

    /// What the placeholder is, in words, for the legend the model reads.
    pub(crate) fn noun(self) -> &'static str {
        match self {
            Kind::Email => "an email address",
            Kind::Phone => "a phone number",
            Kind::Ssn => "a government identity number",
            Kind::Card => "a payment card number",
            Kind::Iban => "a bank account number",
            Kind::Ip => "an IP address",
            Kind::Url => "a personal link",
            Kind::Address => "a street address",
            Kind::Postcode => "a postal code",
            Kind::Date => "a date",
            Kind::Id => "an identifying number",
            Kind::Term => "one of the user's own identifying details (a name, employer or place)",
            Kind::Person => "a person",
            Kind::Org => "an organisation",
            Kind::Place => "a place",
        }
    }

    /// A label from an external name detector ("person", "employer", ...) to a kind.
    pub fn from_label(s: &str) -> Option<Kind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "person" | "name" => Some(Kind::Person),
            "org" | "organisation" | "organization" | "employer" | "company" => Some(Kind::Org),
            "place" | "location" | "address" => Some(Kind::Place),
            "id" | "identifier" | "number" => Some(Kind::Id),
            _ => None,
        }
    }
}
