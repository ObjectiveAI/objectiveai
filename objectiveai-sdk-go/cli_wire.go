package objectiveai

import "encoding/json"

// cliWire lowers a typed leaf request to its JSON wire map so the generated
// execute functions can apply pre-call mutations (jq/transform/stream) and
// inject the `path_type` discriminator before handing it to a CommandExecutor.
// It is the Go analogue of pydantic's `model_dump(mode="json", exclude_none=True)`:
// nil top-level fields are dropped so the wire stays sparse (the CLI tolerates
// null Options regardless).
func cliWire(request any) (map[string]any, error) {
	payload, err := json.Marshal(request)
	if err != nil {
		return nil, err
	}
	var wire map[string]any
	if err := json.Unmarshal(payload, &wire); err != nil {
		return nil, err
	}
	for k, v := range wire {
		if v == nil {
			delete(wire, k)
		}
	}
	return wire, nil
}

// cliAdvanced returns a shallow copy of the wire's `dangerous_advanced` object
// (or an empty map), so the generated stream-flag mutation can edit it without
// aliasing the original.
func cliAdvanced(wire map[string]any) map[string]any {
	adv := map[string]any{}
	if existing, ok := wire["dangerous_advanced"].(map[string]any); ok {
		for k, v := range existing {
			adv[k] = v
		}
	}
	return adv
}
