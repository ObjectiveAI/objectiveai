package objectiveai

import (
	"encoding/json"
	"fmt"
)

// Error implements the error interface for ErrorResponseError.
func (e *ErrorResponseError) Error() string {
	data, _ := json.Marshal(e)
	return fmt.Sprintf("objectiveai: %s", string(data))
}
