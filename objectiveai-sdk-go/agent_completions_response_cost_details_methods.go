package objectiveai

// Push accumulates another CostDetails into this one.
func (v *AgentCompletionsResponseCostDetails) Push(other *AgentCompletionsResponseCostDetails) {
	v.UpstreamInferenceCost += other.UpstreamInferenceCost
	v.UpstreamUpstreamInferenceCost += other.UpstreamUpstreamInferenceCost
}
