use serde::{Deserialize, Serialize};

/// An authenticated Axiom user.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct User {
    /// The user's unique identifier.
    pub id: String,
    /// The user's name.
    pub name: String,
    /// The user's email address.
    pub emails: Vec<String>,
}
