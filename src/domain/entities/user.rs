use std::fmt;

/// Represents a user in the system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_bot: bool,
}

impl User {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            username: None,
            first_name: None,
            last_name: None,
            is_bot: false,
        }
    }

    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn with_name(mut self, first: impl Into<String>, last: Option<impl Into<String>>) -> Self {
        self.first_name = Some(first.into());
        self.last_name = last.map(|l| l.into());
        self
    }

    pub fn display_name(&self) -> String {
        if let Some(ref username) = self.username {
            username.clone()
        } else if let Some(ref first) = self.first_name {
            if let Some(ref last) = self.last_name {
                format!("{} {}", first, last)
            } else {
                first.clone()
            }
        } else {
            self.id.clone()
        }
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
