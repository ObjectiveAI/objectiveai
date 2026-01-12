use wasm_bindgen::prelude::*;

#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn validateEnsembleLlm(llm: JsValue) -> Result<JsValue, JsValue> {
    let llm: objectiveai::ensemble_llm::EnsembleLlmBase =
        serde_wasm_bindgen::from_value(llm)?;
    let llm: objectiveai::ensemble_llm::EnsembleLlm =
        llm.try_into().map_err(|e: String| JsValue::from_str(&e))?;
    let llm: JsValue = serde_wasm_bindgen::to_value(&llm)?;
    Ok(llm)
}

#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn validateEnsemble(ensemble: JsValue) -> Result<JsValue, JsValue> {
    let ensemble: objectiveai::ensemble::EnsembleBase =
        serde_wasm_bindgen::from_value(ensemble)?;
    let ensemble: objectiveai::ensemble::Ensemble = ensemble
        .try_into()
        .map_err(|e: String| JsValue::from_str(&e))?;
    let ensemble: JsValue = serde_wasm_bindgen::to_value(&ensemble)?;
    Ok(ensemble)
}
