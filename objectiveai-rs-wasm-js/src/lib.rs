#![allow(non_snake_case)]
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn validateEnsembleLlm(llm: JsValue) -> Result<JsValue, JsValue> {
    // deserialize
    let llm_base: objectiveai::ensemble_llm::EnsembleLlmBase =
        serde_wasm_bindgen::from_value(llm)?;
    // prepare, validate, and compute ID
    let llm: objectiveai::ensemble_llm::EnsembleLlm = llm_base
        .try_into()
        .map_err(|e: String| JsValue::from_str(&e))?;
    // serialize
    let llm: JsValue = serde_wasm_bindgen::to_value(&llm)?;
    Ok(llm)
}

#[wasm_bindgen]
pub fn validateEnsemble(ensemble: JsValue) -> Result<JsValue, JsValue> {
    // deserialize
    let ensemble_base: objectiveai::ensemble::EnsembleBase =
        serde_wasm_bindgen::from_value(ensemble)?;
    // prepare, validate, and compute ID
    let ensemble: objectiveai::ensemble::Ensemble = ensemble_base
        .try_into()
        .map_err(|e: String| JsValue::from_str(&e))?;
    // serialize
    let ensemble: JsValue = serde_wasm_bindgen::to_value(&ensemble)?;
    Ok(ensemble)
}

#[wasm_bindgen]
pub fn compileFunctionTasks(
    function: JsValue,
    input: JsValue,
) -> Result<JsValue, JsValue> {
    // deserialize
    let function: objectiveai::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai::functions::expression::Input =
        serde_wasm_bindgen::from_value(input)?;
    // compile tasks
    let tasks = function
        .compile_tasks(&input)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    let tasks: JsValue = serde_wasm_bindgen::to_value(&tasks)?;
    Ok(tasks)
}

#[wasm_bindgen]
pub fn compileFunctionOutput(
    function: JsValue,
    input: JsValue,
    task_outputs: JsValue,
) -> Result<JsValue, JsValue> {
    // deserialize
    let function: objectiveai::functions::Function =
        serde_wasm_bindgen::from_value(function)?;
    let input: objectiveai::functions::expression::Input =
        serde_wasm_bindgen::from_value(input)?;
    let task_outputs: Vec<
        Option<objectiveai::functions::expression::TaskOutput<'static>>,
    > = serde_wasm_bindgen::from_value(task_outputs)?;
    // compile output
    let output = function
        .compile_output(&input, &task_outputs)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    // serialize
    let output: JsValue = serde_wasm_bindgen::to_value(&output)?;
    Ok(output)
}

#[wasm_bindgen]
pub fn promptId(prompt: JsValue) -> Result<String, JsValue> {
    // deserialize
    let mut prompt: Vec<objectiveai::chat::completions::request::Message> =
        serde_wasm_bindgen::from_value(prompt)?;
    // prepare and compute ID
    objectiveai::chat::completions::request::prompt::prepare(&mut prompt);
    let id = objectiveai::chat::completions::request::prompt::id(&prompt);
    Ok(id)
}

#[wasm_bindgen]
pub fn toolsId(tools: JsValue) -> Result<String, JsValue> {
    // deserialize
    let tools: Vec<objectiveai::chat::completions::request::Tool> =
        serde_wasm_bindgen::from_value(tools)?;
    // compute ID
    let id = objectiveai::chat::completions::request::tools::id(&tools);
    Ok(id)
}

#[wasm_bindgen]
pub fn vectorResponseId(response: JsValue) -> Result<String, JsValue> {
    // deserialize
    let mut response: objectiveai::chat::completions::request::RichContent =
        serde_wasm_bindgen::from_value(response)?;
    // prepare and compute ID
    response.prepare();
    let id = response.id();
    Ok(id)
}
