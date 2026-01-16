import { validateEnsembleLlm } from "#wasm-loader";
import { EnsembleLlm, EnsembleLlmBase } from "./ensemble_llm";

export function validate(ensemble: EnsembleLlmBase): EnsembleLlm {
  return validateEnsembleLlm(ensemble) as EnsembleLlm;
}
