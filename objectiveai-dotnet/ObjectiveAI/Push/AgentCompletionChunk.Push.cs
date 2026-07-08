using static ObjectiveAI.PushUtils;

namespace ObjectiveAI.Agent.Completions.Response.Streaming;

public partial class AgentCompletionChunk
{
    public void Push(AgentCompletionChunk other)
    {
        PushByNullableIndex(
            Messages,
            other.Messages,
            m => m.Index(),
            (a, b) => a.Push(b)
        );
        var usage = Usage;
        PushOption(ref usage, other.Usage, (a, b) => a.Push(b));
        Usage = usage;
        Error = PushReplace(Error, other.Error);
        Continuation = PushReplace(Continuation, other.Continuation);
        // agent_inline: first chunk wins (rides only the completion's
        // first chunk; never overwritten)
        AgentInline ??= other.AgentInline;
        // id, created, object, upstream: immutable
    }
}
