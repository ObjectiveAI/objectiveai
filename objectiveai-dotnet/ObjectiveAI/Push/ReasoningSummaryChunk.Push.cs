using ObjectiveAI.Agent.Completions.Response.Streaming;
using static ObjectiveAI.PushUtils;

namespace ObjectiveAI.Functions.Executions.Response.Streaming;

public partial class ReasoningSummaryChunk
{
    public void Push(ReasoningSummaryChunk other)
    {
        PushByNullableIndex(
            Messages,
            other.Messages,
            m => m.Index(),
            (a, b) => a.Push(b)
        );
        Error = PushReplace(Error, other.Error);
        Continuation = PushReplace(Continuation, other.Continuation);
        // agent_inline: first chunk wins (rides only the completion's
        // first chunk; never overwritten)
        AgentInline ??= other.AgentInline;
        var usage = Usage;
        PushOption(ref usage, other.Usage, (a, b) => a.Push(b));
        Usage = usage;
        // id, created, object, upstream: immutable
    }
}
