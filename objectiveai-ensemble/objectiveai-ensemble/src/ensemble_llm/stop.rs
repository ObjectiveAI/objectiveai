use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    Strings(Vec<String>),
}

impl Stop {
    pub fn prepare(self) -> Option<Self> {
        if let Stop::Strings(mut strings) = self {
            if strings.is_empty() {
                None
            } else if strings.len() == 1 {
                Some(Stop::String(strings.into_iter().next().unwrap()))
            } else {
                strings.sort();
                strings.dedup();
                Some(Stop::Strings(strings))
            }
        } else {
            Some(self)
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self {
            Stop::String(string) if string.is_empty() => {
                Err("`stop` string cannot be empty".to_string())
            }
            Stop::Strings(strings) if strings.iter().any(|s| s.is_empty()) => {
                Err("`stop` strings cannot be empty".to_string())
            }
            _ => Ok(()),
        }
    }
}
