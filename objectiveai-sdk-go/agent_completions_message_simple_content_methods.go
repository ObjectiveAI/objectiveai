package objectiveai

// Push accumulates another SimpleContent into this one.
// Union dispatch: text+text -> concat, text+parts -> convert, parts+text -> append, parts+parts -> extend.
func (v *AgentCompletionsMessageSimpleContent) Push(other *AgentCompletionsMessageSimpleContent) {
	selfIsText := v.Text != nil
	otherIsText := other.Text != nil

	switch {
	case selfIsText && otherIsText:
		s := AgentCompletionsMessageSimpleContentText(string(*v.Text) + string(*other.Text))
		v.Text = &s

	case selfIsText && !otherIsText:
		textPart := AgentCompletionsMessageSimpleContentPart{
			Text: string(*v.Text),
			Type: "text",
		}
		parts := make(AgentCompletionsMessageSimpleContentParts, 0, 1+len(*other.Parts))
		parts = append(parts, textPart)
		parts = append(parts, *other.Parts...)
		v.Text = nil
		v.Parts = &parts

	case !selfIsText && otherIsText:
		if other.Text != nil && string(*other.Text) != "" {
			textPart := AgentCompletionsMessageSimpleContentPart{
				Text: string(*other.Text),
				Type: "text",
			}
			*v.Parts = append(*v.Parts, textPart)
		}

	default:
		if other.Parts != nil && len(*other.Parts) > 0 {
			*v.Parts = append(*v.Parts, *other.Parts...)
		}
	}
}
