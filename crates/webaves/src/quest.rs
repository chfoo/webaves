//! Representation of work units for retrieving resources on the internet.

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// ID of the quest.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestId(Uuid);

impl Display for QuestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Represents a work unit or task for retrieving a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    /// ID of the quest.
    pub id: QuestId,

    /// URL of the resource to be fetched.
    pub url: Url,

    /// The previous quest that invoked this quest.
    pub parent: Option<Box<Quest>>,

    /// The ancestry count of the quest.
    ///
    /// A quest with no parent (root) is depth 0, the child is 1,
    /// the grandchild is 2, and so on.
    pub depth: u64,

    /// Protocol-specific parameters.
    pub protocol_parameters: ProtocolParameters,
}

/// Protocol-specific parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolParameters {
    /// No protocol-specific parameters.
    None,

    /// HTTP parameters.
    Http(HttpQuest),
}

impl Default for ProtocolParameters {
    fn default() -> Self {
        Self::None
    }
}

/// HTTP request contextual parameters.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HttpQuest {
    /// Whether to this is an object embedded in a web page such as an image or stylesheet.
    pub is_object: Option<bool>,

    /// Expected MIME-type.
    pub media_type: Option<String>,

    /// URL to be sent as the referrer URL.
    pub referrer_url: Option<Url>,
}
