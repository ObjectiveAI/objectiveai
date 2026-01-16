import { validateEnsemble } from "#wasm-loader";
import { Ensemble, EnsembleBase } from "./ensemble";

export function validate(ensemble: EnsembleBase): Ensemble {
  return validateEnsemble(ensemble) as Ensemble;
}
