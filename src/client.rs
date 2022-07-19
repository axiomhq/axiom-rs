//! The top-level client for the Axiom API.
use std::env;

use crate::{
    datasets,
    error::{Error, Result},
    http, is_personal_token, users, version,
};

/// Cloud URL is the URL for Axiom Cloud.
static CLOUD_URL: &str = "https://cloud.axiom.co";

/// The client is the entrypoint of the whole SDK.
///
/// You can create it using [`Client::builder`] or [`Client::new`].
///
/// # Examples
/// ```
/// use axiom_rs::{Client, Error};
///
/// fn main() -> Result<(), Error> {
///     // Create a new client and get the token, url and (if necesary) org id
///     // from the environment variables AXIOM_TOKEN, AXIOM_URL and AXIOM_ORG_ID.
///     let client = Client::new()?;
///
///     // Set all available options. Unset options fall back to environment
///     // variables.
///     let client = Client::builder()
///         .with_token("my-token")
///         .with_url("http://example.org")
///         .with_org_id("my-org-id")
///         .build()?;
///
///     Ok(())
/// }
/// ```
pub struct Client {
    url: String,
    pub datasets: datasets::Client,
    users: users::Client,
    version: version::Client,
}

impl Client {
    /// Creates a new client. If you want to configure it, use [`Client::builder`].
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Create a new client using a builder.
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Get the url (cloned).
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Get the server and client versions.
    pub async fn version(&self) -> Result<Version> {
        Ok(Version {
            server: self.version.get().await?,
            client: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Make sure the client can properly authenticate against the configured
    /// Axiom deployment.
    pub async fn validate_credentials(&self) -> Result<()> {
        self.users.current().await?;
        Ok(())
    }
}

/// The server and client versions.
pub struct Version {
    pub server: String,
    pub client: String,
}

/// This builder is used to create a new client.
pub struct Builder {
    env_fallback: bool,
    url: Option<String>,
    token: Option<String>,
    org_id: Option<String>,
}

impl Builder {
    /// Create a new builder.
    fn new() -> Self {
        Self {
            env_fallback: true,
            url: None,
            token: None,
            org_id: None,
        }
    }

    /// Don't fall back to environment variables.
    pub fn no_env(mut self) -> Self {
        self.env_fallback = false;
        self
    }

    /// Add a token to the client. If this is not set, the token will be read
    /// from the environment variable `AXIOM_TOKEN`.
    pub fn with_token<S: Into<String>>(mut self, token: S) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Add an URL to the client. If this is not set, the URL will be read
    /// from the environment variable `AXIOM_URL`. If that is empty as well,
    /// it will fall back to Axiom Cloud.
    pub fn with_url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Add an organization ID to the client. If this is not set, the
    /// organization ID will be read from the environment variable `AXIOM_ORG_ID`.
    pub fn with_org_id<S: Into<String>>(mut self, org_id: S) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client> {
        let env_fallback = self.env_fallback;

        let token = self
            .token
            .or_else(|| {
                if env_fallback {
                    env::var("AXIOM_TOKEN").ok()
                } else {
                    None
                }
            })
            .ok_or(Error::MissingToken)?;

        let url = self
            .url
            .or_else(|| {
                if env_fallback {
                    env::var("AXIOM_URL").ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| CLOUD_URL.to_string());

        let org_id = self.org_id.or_else(|| {
            if env_fallback {
                env::var("AXIOM_ORG_ID").ok()
            } else {
                None
            }
        });

        // On Cloud you need an Org ID for Personal Tokens.
        if url == CLOUD_URL && org_id.is_none() && is_personal_token(&token) {
            return Err(Error::MissingOrgId);
        }

        let http_client = http::Client::new(url.clone(), token, org_id)?;

        Ok(Client {
            url,
            datasets: datasets::Client::new(http_client.clone()),
            users: users::Client::new(http_client.clone()),
            version: version::Client::new(http_client),
        })
    }
}
