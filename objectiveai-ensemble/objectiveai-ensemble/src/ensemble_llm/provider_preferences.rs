use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub struct ProviderPreferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantizations: Option<Vec<ProviderPreferencesQuantization>>,
}

impl ProviderPreferences {
    pub fn prepare(mut self) -> Option<Self> {
        if let Some(true) = self.allow_fallbacks {
            self.allow_fallbacks = None;
        }
        if let Some(false) = self.require_parameters {
            self.require_parameters = None;
        }
        if let Some(order) = &mut self.order {
            if order.is_empty() {
                self.order = None;
            } else {
                let mut dedup = HashSet::with_capacity(order.len());
                order.retain(|provider| dedup.insert(provider.clone()));
            }
        }
        if let Some(only) = &mut self.only {
            if only.is_empty() {
                self.only = None;
            } else {
                only.sort();
                only.dedup();
            }
        }
        if let Some(ignore) = &mut self.ignore {
            if ignore.is_empty() {
                self.ignore = None;
            } else {
                ignore.sort();
                ignore.dedup();
            }
        }
        if let Some(quantizations) = &mut self.quantizations {
            if quantizations.is_empty() {
                self.quantizations = None;
            } else {
                quantizations.sort();
                quantizations.dedup();
            }
        }
        if self.allow_fallbacks.is_some()
            || self.require_parameters.is_some()
            || self.order.is_some()
            || self.only.is_some()
            || self.ignore.is_some()
            || self.quantizations.is_some()
        {
            Some(self)
        } else {
            None
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.order.iter().any(|s| s.is_empty()) {
            Err("`provider.order` strings cannot be empty".to_string())
        } else if self.only.iter().any(|s| s.is_empty()) {
            Err("`provider.only` strings cannot be empty".to_string())
        } else if self.ignore.iter().any(|s| s.is_empty()) {
            Err("`provider.ignore` strings cannot be empty".to_string())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPreferencesQuantization {
    Int4,
    Int8,
    Fp4,
    Fp6,
    Fp8,
    Fp16,
    Bf16,
    Fp32,
    Unknown,
}

impl PartialOrd for ProviderPreferencesQuantization {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some((*self as u16).cmp(&(*other as u16)))
    }
}

impl Ord for ProviderPreferencesQuantization {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
