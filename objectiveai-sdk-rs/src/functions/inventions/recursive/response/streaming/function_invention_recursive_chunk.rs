use crate::agent;
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.recursive.response.streaming.FunctionInventionRecursiveChunk")]
pub struct FunctionInventionRecursiveChunk {
    pub id: String,
    pub inventions: Vec<super::FunctionInventionChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub inventions_errors: Option<bool>,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub object: super::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}

impl AgentCompletionIds for FunctionInventionRecursiveChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inventions.iter().flat_map(|i| i.agent_completion_ids())
    }
}

impl FunctionInventionRecursiveChunk {
    /// Flat-maps message rows from every wrapped non-recursive invention. Lazy.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_ {
        self.inventions
            .iter()
            .flat_map(|i| i.produce_message_rows())
    }
}

impl FunctionInventionRecursiveChunk {
    /// Yields each inner error from the recursive chunk's wrapped
    /// non-recursive `FunctionInventionChunk`s.
    ///
    /// For each wrapped invention:
    /// 1. If the wrapped invention has its own `.error`, yields one item
    ///    with `agent_completion_index: None`.
    /// 2. Then yields each item from the wrapped invention's
    ///    `inner_errors()`, re-tagged with this wrapper's
    ///    `function_invention_index` and `agent_completion_index:
    ///    Some(non_recursive.agent_completion_index)`.
    ///
    /// Lazy and zero-allocation; collect with `.collect::<Vec<_>>()` if
    /// you need to retain the items past the chunk's lifetime.
    pub fn inner_errors(&self) -> impl Iterator<Item = super::InnerError<'_>> {
        self.inventions.iter().flat_map(|wrapper| {
            let function_invention_index = wrapper.index;
            let own = wrapper.inner.error.as_ref().map(move |error| super::InnerError {
                function_invention_index,
                agent_completion_index: None,
                error: std::borrow::Cow::Borrowed(error),
            });
            let nested = wrapper.inner.inner_errors().map(move |non_recursive| {
                super::InnerError {
                    function_invention_index,
                    agent_completion_index: Some(non_recursive.agent_completion_index),
                    error: non_recursive.error,
                }
            });
            own.into_iter().chain(nested)
        })
    }

    pub fn push(
        &mut self,
        FunctionInventionRecursiveChunk {
            inventions,
            inventions_errors,
            usage,
            ..
        }: &FunctionInventionRecursiveChunk,
    ) {
        self.push_inventions(inventions);
        if let Some(true) = inventions_errors {
            self.inventions_errors = Some(true);
        }
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
    }

    fn push_inventions(
        &mut self,
        other_inventions: &[super::FunctionInventionChunk],
    ) {
        fn push_invention(
            inventions: &mut Vec<super::FunctionInventionChunk>,
            other: &super::FunctionInventionChunk,
        ) {
            fn find_invention(
                inventions: &mut Vec<super::FunctionInventionChunk>,
                index: u64,
            ) -> Option<&mut super::FunctionInventionChunk> {
                for invention in inventions {
                    if invention.index == index {
                        return Some(invention);
                    }
                }
                None
            }
            if let Some(existing) = find_invention(inventions, other.index) {
                existing.push(other);
            } else {
                inventions.push(other.clone());
            }
        }
        for other in other_inventions {
            push_invention(&mut self.inventions, other);
        }
    }

    /// Produces the [`LogFile`]s for the log file structure.
    ///
    /// Returns `(reference, files)`. All paths relative to `logs/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> Option<(crate::filesystem::logs::LogReference, Vec<crate::filesystem::logs::LogFile>)> {
        use crate::filesystem::logs::{LogFile, LogReference};
        const ROUTE: &str = "functions/inventions/recursive/response";

        let id = &self.id;
        if id.is_empty() {
            return None;
        }

        let mut files: Vec<LogFile> = Vec::new();
        let mut invention_refs:
            Vec<crate::filesystem::logs::indexed_reference::LogReference> = Vec::new();

        for invention in &self.inventions {
            let (reference, invention_files) = invention.produce_files();
            invention_refs.push(reference);
            files.extend(invention_files);
        }

        let log = super::FunctionInventionRecursiveChunkLog {
            id: self.id.clone(),
            inventions: invention_refs,
            inventions_errors: self.inventions_errors,
            created: self.created,
            object: self.object,
            usage: self.usage.clone(),
        };

        let root_file = LogFile {
            route: ROUTE.to_string(),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&log).unwrap(),
        };
        let reference = LogReference::new(root_file.path());
        files.push(root_file);

        Some((reference, files))
    }
}
