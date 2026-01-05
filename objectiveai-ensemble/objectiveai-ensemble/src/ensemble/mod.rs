use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnsembleBase {
    pub llms: Vec<super::EnsembleLlmBaseWithFallbacksAndCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ensemble {
    pub id: String,
    pub llms: Vec<super::EnsembleLlmWithFallbacksAndCount>,
}

impl TryFrom<EnsembleBase> for Ensemble {
    type Error = String;
    fn try_from(
        EnsembleBase { llms: base_llms }: EnsembleBase,
    ) -> Result<Self, Self::Error> {
        // convert all base LLMs and merge duplicates
        let mut llms_with_full_id: IndexMap<
            String,
            super::EnsembleLlmWithFallbacksAndCount,
        > = IndexMap::with_capacity(base_llms.len());
        let mut count = 0;
        for base_llm in base_llms {
            match base_llm.count {
                Some(0) => continue,
                Some(n) => count += n,
                None => count += 1,
            }
            let llm: super::EnsembleLlmWithFallbacksAndCount =
                base_llm.try_into()?;
            let full_id = llm.full_id();
            match llms_with_full_id.get_mut(&full_id) {
                Some(existing_llm) => {
                    match (&mut existing_llm.count, llm.count) {
                        (None, None) => {
                            existing_llm.count = Some(2);
                        }
                        (None, Some(new_count)) => {
                            existing_llm.count = Some(1 + new_count);
                        }
                        (Some(existing_count), None) => {
                            *existing_count += 1;
                        }
                        (Some(existing_count), Some(new_count)) => {
                            *existing_count += new_count;
                        }
                    }
                }
                None => {
                    llms_with_full_id.insert(full_id, llm);
                }
            }
        }

        // validate count
        if count == 0 || count > 128 {
            return Err(
                "`ensemble.llms` must contain between 1 and 128 total LLMs"
                    .to_string(),
            );
        }

        // sort by full_id to ensure deterministic order
        llms_with_full_id.sort_unstable_keys();

        // compute ensemble ID
        let mut hasher = XxHash3_128::with_seed(0);
        for (full_id, llm) in &llms_with_full_id {
            hasher.write(full_id.as_bytes());
            let count_bytes = if let Some(count) = llm.count {
                count.to_le_bytes()
            } else {
                [1, 0, 0, 0, 0, 0, 0, 0]
            };
            hasher.write(&count_bytes);
        }
        let id = format!("{:0>22}", base62::encode(hasher.finish_128()));

        // collect LLMs
        let llms = llms_with_full_id.into_values().collect::<Vec<_>>();

        // return ensemble
        Ok(Ensemble { id, llms })
    }
}
