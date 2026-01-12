#![allow(non_snake_case)]

pub mod ensemble_llm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn validate(llm: JsValue) -> Result<JsValue, JsValue> {
        // deserialize
        let llm_base: objectiveai::ensemble_llm::EnsembleLlmBase =
            serde_wasm_bindgen::from_value(llm)?;
        // prepare, validate, and compute ID
        let llm: objectiveai::ensemble_llm::EnsembleLlm =
            llm_base
                .try_into()
                .map_err(|e: String| JsValue::from_str(&e))?;
        // serialize
        let llm: JsValue = serde_wasm_bindgen::to_value(&llm)?;
        Ok(llm)
    }
}

pub mod ensemble {
    use wasm_bindgen::prelude::*;

    #[allow(non_snake_case)]
    #[wasm_bindgen]
    pub fn validate(ensemble: JsValue) -> Result<JsValue, JsValue> {
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
}

pub mod functions {
    use wasm_bindgen::prelude::*;

    #[allow(non_snake_case)]
    #[wasm_bindgen]
    pub fn compileTasks(
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

    #[allow(non_snake_case)]
    #[wasm_bindgen]
    pub fn compileOutput(
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
}
