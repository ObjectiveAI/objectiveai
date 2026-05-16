package objectiveai

// Push accumulates another RichContent into this one.
// Union dispatch: text+text -> concat, text+parts -> convert, parts+text -> append, parts+parts -> extend.
func (v *AgentCompletionsMessageRichContent) Push(other *AgentCompletionsMessageRichContent) {
	selfIsText := v.Text != nil
	otherIsText := other.Text != nil

	switch {
	case selfIsText && otherIsText:
		// text + text -> concatenate
		s := AgentCompletionsMessageRichContentText(string(*v.Text) + string(*other.Text))
		v.Text = &s

	case selfIsText && !otherIsText:
		// text + parts -> convert self to parts, extend
		textStr := string(*v.Text)
		textPart := AgentCompletionsMessageRichContentPart{
			Text: &AgentCompletionsMessageRichContentPartText{
				Text: textStr,
				Type: "text",
			},
		}
		parts := make(AgentCompletionsMessageRichContentParts, 0, 1+len(*other.Parts))
		parts = append(parts, textPart)
		parts = append(parts, *other.Parts...)
		v.Text = nil
		v.Parts = &parts

	case !selfIsText && otherIsText:
		// parts + text -> append text as new part
		if other.Text != nil && string(*other.Text) != "" {
			textStr := string(*other.Text)
			textPart := AgentCompletionsMessageRichContentPart{
				Text: &AgentCompletionsMessageRichContentPartText{
					Text: textStr,
					Type: "text",
				},
			}
			*v.Parts = append(*v.Parts, textPart)
		}

	default:
		// parts + parts -> extend
		if other.Parts != nil && len(*other.Parts) > 0 {
			*v.Parts = append(*v.Parts, *other.Parts...)
		}
	}
}
