use serde::{Deserialize, Serialize};

/// An authenticated Axiom user.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct AuthenticatedUser {
    pub id: String,
    pub name: String,
    pub emails: Vec<String>,
}
