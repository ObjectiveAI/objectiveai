import { mapsToRecords } from "src/mapsToRecords";
import { validateEnsembleLlm } from "../wasm/loader.js";
import { EnsembleLlm, EnsembleLlmBase } from "./ensemble_llm";

export function validate(ensemble: EnsembleLlmBase): EnsembleLlm {
  const value = validateEnsembleLlm(ensemble);
  const unmapped = mapsToRecords(value);
  return unmapped as EnsembleLlm;
}
