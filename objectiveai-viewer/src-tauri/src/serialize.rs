//! Passthrough serialization of the SDK's distributed
//! `ListenerExecution` tree: [`into_serialized`] flattens any variant
//! back to wire-shaped JSON — the serialized request, the producer's
//! `AgentArguments`, and the response as a boxed future/stream of
//! serialized items — so the viewer's daemon client
//! ([`crate::daemon_ws`]) re-packages runs into the standard
//! broadcast envelope without a hand-written per-leaf match.
//!
//! Serialization uses each type's own `Serialize` impl, so fidelity
//! matches what the cli itself prints (in-band `cli::Error` items
//! stay on the `Err` side, exactly as they arrived).
//!
//! Mechanically authored from the SDK's command tree — regenerate
//! alongside the SDK's leaf/branch `ListenerExecution` items when the
//! command tree grows.

use futures::future::BoxFuture;
use futures::stream::BoxStream;

use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::ListenerExecution;

/// A [`ListenerExecution`] flattened for generic consumption — the
/// same three things every envelope carries, with the typed pieces
/// serialized back to JSON.
pub struct SerializedListenerExecution {
    /// The run's request, serialized.
    pub request: serde_json::Value,
    /// The producer's identity.
    pub agent_arguments: AgentArguments,
    /// The response — unary future or item stream, items serialized.
    pub response: SerializedListenerResponse,
}

/// The serialized response side of a run: in-band [`objectiveai_sdk::cli::Error`]
/// lines stay on the `Err` side, exactly as they arrived.
pub enum SerializedListenerResponse {
    /// A unary run's single response.
    Unary(BoxFuture<'static, Result<serde_json::Value, objectiveai_sdk::cli::Error>>),
    /// A streaming run's items.
    Stream(BoxStream<'static, Result<serde_json::Value, objectiveai_sdk::cli::Error>>),
}

/// Flatten one envelope for generic consumption — see the module
/// docs.
pub(crate) fn into_serialized(execution: ListenerExecution) -> SerializedListenerExecution {
    match execution {
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Enqueue(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::EnqueueRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::EnqueueResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Get(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::GetRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::GetResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Instances(objectiveai_sdk::cli::command::agents::instances::ListenerExecution::Get(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Instances(objectiveai_sdk::cli::command::agents::instances::ListenerExecution::GetRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Instances(objectiveai_sdk::cli::command::agents::instances::ListenerExecution::GetResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Instances(objectiveai_sdk::cli::command::agents::instances::ListenerExecution::List(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Instances(objectiveai_sdk::cli::command::agents::instances::ListenerExecution::ListRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Instances(objectiveai_sdk::cli::command::agents::instances::ListenerExecution::ListResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::Attach(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::AttachRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::AttachResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::Detach(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::DetachRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::DetachResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::List(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::ListRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::agents::laboratories::ListenerExecution::ListResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::List(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::ListRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::ListResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::List(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::ListRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::ListResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::Open(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::OpenRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::OpenResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::Subscribe(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::SubscribeRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::SubscribeResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::TokenUsage(objectiveai_sdk::cli::command::agents::logs::token_usage::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::TokenUsage(objectiveai_sdk::cli::command::agents::logs::token_usage::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::TokenUsage(objectiveai_sdk::cli::command::agents::logs::token_usage::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::TokenUsage(objectiveai_sdk::cli::command::agents::logs::token_usage::ListenerExecution::Subscribe(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::TokenUsage(objectiveai_sdk::cli::command::agents::logs::token_usage::ListenerExecution::SubscribeRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Logs(objectiveai_sdk::cli::command::agents::logs::ListenerExecution::TokenUsage(objectiveai_sdk::cli::command::agents::logs::token_usage::ListenerExecution::SubscribeResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Resources(objectiveai_sdk::cli::command::agents::mcp::resources::ListenerExecution::List(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Resources(objectiveai_sdk::cli::command::agents::mcp::resources::ListenerExecution::ListRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Resources(objectiveai_sdk::cli::command::agents::mcp::resources::ListenerExecution::ListResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Resources(objectiveai_sdk::cli::command::agents::mcp::resources::ListenerExecution::Read(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Resources(objectiveai_sdk::cli::command::agents::mcp::resources::ListenerExecution::ReadRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Resources(objectiveai_sdk::cli::command::agents::mcp::resources::ListenerExecution::ReadResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Servers(objectiveai_sdk::cli::command::agents::mcp::servers::ListenerExecution::List(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Servers(objectiveai_sdk::cli::command::agents::mcp::servers::ListenerExecution::ListRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Servers(objectiveai_sdk::cli::command::agents::mcp::servers::ListenerExecution::ListResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Tools(objectiveai_sdk::cli::command::agents::mcp::tools::ListenerExecution::Call(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Tools(objectiveai_sdk::cli::command::agents::mcp::tools::ListenerExecution::CallRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Tools(objectiveai_sdk::cli::command::agents::mcp::tools::ListenerExecution::CallResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Tools(objectiveai_sdk::cli::command::agents::mcp::tools::ListenerExecution::List(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Tools(objectiveai_sdk::cli::command::agents::mcp::tools::ListenerExecution::ListRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Mcp(objectiveai_sdk::cli::command::agents::mcp::ListenerExecution::Tools(objectiveai_sdk::cli::command::agents::mcp::tools::ListenerExecution::ListResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Message(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::MessageRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::MessageResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Publish(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::PublishRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::PublishResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::Delete(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::DeleteRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::DeleteResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::Deliver(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::DeliverRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::DeliverResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::List(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::ListRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::ListResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::Open(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::OpenRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Queue(objectiveai_sdk::cli::command::agents::queue::ListenerExecution::OpenResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Spawn(objectiveai_sdk::cli::command::agents::spawn::ListenerExecutionVariant::Execution(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Spawn(objectiveai_sdk::cli::command::agents::spawn::ListenerExecutionVariant::Streaming(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::SpawnRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::SpawnResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Tags(objectiveai_sdk::cli::command::agents::tags::ListenerExecution::Apply(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Tags(objectiveai_sdk::cli::command::agents::tags::ListenerExecution::ApplyRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Tags(objectiveai_sdk::cli::command::agents::tags::ListenerExecution::ApplyResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Tags(objectiveai_sdk::cli::command::agents::tags::ListenerExecution::Lookup(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Tags(objectiveai_sdk::cli::command::agents::tags::ListenerExecution::LookupRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Tags(objectiveai_sdk::cli::command::agents::tags::ListenerExecution::LookupResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::Wait(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::WaitRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Agents(objectiveai_sdk::cli::command::agents::ListenerExecution::WaitResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Address(objectiveai_sdk::cli::command::api::config::address::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Address(objectiveai_sdk::cli::command::api::config::address::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Address(objectiveai_sdk::cli::command::api::config::address::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Address(objectiveai_sdk::cli::command::api::config::address::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Address(objectiveai_sdk::cli::command::api::config::address::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Address(objectiveai_sdk::cli::command::api::config::address::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::BackoffMaxElapsedTimeMs(objectiveai_sdk::cli::command::api::config::backoff_max_elapsed_time_ms::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorEmail(objectiveai_sdk::cli::command::api::config::commit_author_email::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorEmail(objectiveai_sdk::cli::command::api::config::commit_author_email::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorEmail(objectiveai_sdk::cli::command::api::config::commit_author_email::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorEmail(objectiveai_sdk::cli::command::api::config::commit_author_email::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorEmail(objectiveai_sdk::cli::command::api::config::commit_author_email::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorEmail(objectiveai_sdk::cli::command::api::config::commit_author_email::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorName(objectiveai_sdk::cli::command::api::config::commit_author_name::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorName(objectiveai_sdk::cli::command::api::config::commit_author_name::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorName(objectiveai_sdk::cli::command::api::config::commit_author_name::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorName(objectiveai_sdk::cli::command::api::config::commit_author_name::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorName(objectiveai_sdk::cli::command::api::config::commit_author_name::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::CommitAuthorName(objectiveai_sdk::cli::command::api::config::commit_author_name::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::Get(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GetRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GetResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GithubAuthorization(objectiveai_sdk::cli::command::api::config::github_authorization::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GithubAuthorization(objectiveai_sdk::cli::command::api::config::github_authorization::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GithubAuthorization(objectiveai_sdk::cli::command::api::config::github_authorization::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GithubAuthorization(objectiveai_sdk::cli::command::api::config::github_authorization::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GithubAuthorization(objectiveai_sdk::cli::command::api::config::github_authorization::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::GithubAuthorization(objectiveai_sdk::cli::command::api::config::github_authorization::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::HttpReferer(objectiveai_sdk::cli::command::api::config::http_referer::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::HttpReferer(objectiveai_sdk::cli::command::api::config::http_referer::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::HttpReferer(objectiveai_sdk::cli::command::api::config::http_referer::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::HttpReferer(objectiveai_sdk::cli::command::api::config::http_referer::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::HttpReferer(objectiveai_sdk::cli::command::api::config::http_referer::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::HttpReferer(objectiveai_sdk::cli::command::api::config::http_referer::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::Add(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::AddRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::AddResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::Del(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::DelRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::DelResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpAuthorization(objectiveai_sdk::cli::command::api::config::mcp_authorization::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpTimeoutMs(objectiveai_sdk::cli::command::api::config::mcp_timeout_ms::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpTimeoutMs(objectiveai_sdk::cli::command::api::config::mcp_timeout_ms::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpTimeoutMs(objectiveai_sdk::cli::command::api::config::mcp_timeout_ms::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpTimeoutMs(objectiveai_sdk::cli::command::api::config::mcp_timeout_ms::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpTimeoutMs(objectiveai_sdk::cli::command::api::config::mcp_timeout_ms::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::McpTimeoutMs(objectiveai_sdk::cli::command::api::config::mcp_timeout_ms::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(objectiveai_sdk::cli::command::api::config::objectiveai_authorization::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(objectiveai_sdk::cli::command::api::config::objectiveai_authorization::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(objectiveai_sdk::cli::command::api::config::objectiveai_authorization::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(objectiveai_sdk::cli::command::api::config::objectiveai_authorization::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(objectiveai_sdk::cli::command::api::config::objectiveai_authorization::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::ObjectiveaiAuthorization(objectiveai_sdk::cli::command::api::config::objectiveai_authorization::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(objectiveai_sdk::cli::command::api::config::openrouter_authorization::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(objectiveai_sdk::cli::command::api::config::openrouter_authorization::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(objectiveai_sdk::cli::command::api::config::openrouter_authorization::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(objectiveai_sdk::cli::command::api::config::openrouter_authorization::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(objectiveai_sdk::cli::command::api::config::openrouter_authorization::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::OpenrouterAuthorization(objectiveai_sdk::cli::command::api::config::openrouter_authorization::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::UserAgent(objectiveai_sdk::cli::command::api::config::user_agent::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::UserAgent(objectiveai_sdk::cli::command::api::config::user_agent::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::UserAgent(objectiveai_sdk::cli::command::api::config::user_agent::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::UserAgent(objectiveai_sdk::cli::command::api::config::user_agent::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::UserAgent(objectiveai_sdk::cli::command::api::config::user_agent::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::UserAgent(objectiveai_sdk::cli::command::api::config::user_agent::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::XTitle(objectiveai_sdk::cli::command::api::config::x_title::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::XTitle(objectiveai_sdk::cli::command::api::config::x_title::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::XTitle(objectiveai_sdk::cli::command::api::config::x_title::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::XTitle(objectiveai_sdk::cli::command::api::config::x_title::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::XTitle(objectiveai_sdk::cli::command::api::config::x_title::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Config(objectiveai_sdk::cli::command::api::config::ListenerExecution::XTitle(objectiveai_sdk::cli::command::api::config::x_title::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Kill(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::KillRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::KillResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::Spawn(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::SpawnRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Api(objectiveai_sdk::cli::command::api::ListenerExecution::SpawnResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Daemon(objectiveai_sdk::cli::command::daemon::ListenerExecution::Kill(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Daemon(objectiveai_sdk::cli::command::daemon::ListenerExecution::KillRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Daemon(objectiveai_sdk::cli::command::daemon::ListenerExecution::KillResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Daemon(objectiveai_sdk::cli::command::daemon::ListenerExecution::Spawn(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Daemon(objectiveai_sdk::cli::command::daemon::ListenerExecution::SpawnRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Daemon(objectiveai_sdk::cli::command::daemon::ListenerExecution::SpawnResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Address(objectiveai_sdk::cli::command::db::config::address::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Address(objectiveai_sdk::cli::command::db::config::address::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Address(objectiveai_sdk::cli::command::db::config::address::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Address(objectiveai_sdk::cli::command::db::config::address::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Address(objectiveai_sdk::cli::command::db::config::address::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Address(objectiveai_sdk::cli::command::db::config::address::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Database(objectiveai_sdk::cli::command::db::config::database::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Database(objectiveai_sdk::cli::command::db::config::database::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Database(objectiveai_sdk::cli::command::db::config::database::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Database(objectiveai_sdk::cli::command::db::config::database::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Database(objectiveai_sdk::cli::command::db::config::database::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Database(objectiveai_sdk::cli::command::db::config::database::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Get(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::GetRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::GetResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Password(objectiveai_sdk::cli::command::db::config::password::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Password(objectiveai_sdk::cli::command::db::config::password::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Password(objectiveai_sdk::cli::command::db::config::password::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Password(objectiveai_sdk::cli::command::db::config::password::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Password(objectiveai_sdk::cli::command::db::config::password::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::Password(objectiveai_sdk::cli::command::db::config::password::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::User(objectiveai_sdk::cli::command::db::config::user::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::User(objectiveai_sdk::cli::command::db::config::user::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::User(objectiveai_sdk::cli::command::db::config::user::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::User(objectiveai_sdk::cli::command::db::config::user::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::User(objectiveai_sdk::cli::command::db::config::user::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Config(objectiveai_sdk::cli::command::db::config::ListenerExecution::User(objectiveai_sdk::cli::command::db::config::user::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Kill(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::KillRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::KillResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Query(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::QueryRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::QueryResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::Spawn(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::SpawnRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Db(objectiveai_sdk::cli::command::db::ListenerExecution::SpawnResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::Standard(objectiveai_sdk::cli::command::functions::execute::standard::ListenerExecutionVariant::Execution(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::Standard(objectiveai_sdk::cli::command::functions::execute::standard::ListenerExecutionVariant::Streaming(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::StandardRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::StandardResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::SwissSystem(objectiveai_sdk::cli::command::functions::execute::swiss_system::ListenerExecutionVariant::Execution(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::SwissSystem(objectiveai_sdk::cli::command::functions::execute::swiss_system::ListenerExecutionVariant::Streaming(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::SwissSystemRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Execute(objectiveai_sdk::cli::command::functions::execute::ListenerExecution::SwissSystemResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Get(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::GetRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::GetResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::List(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::ListRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::ListResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::Get(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::GetRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::GetResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::List(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::ListRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::ListResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::Publish(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::PublishRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Profiles(objectiveai_sdk::cli::command::functions::profiles::ListenerExecution::PublishResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::Publish(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::PublishRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Functions(objectiveai_sdk::cli::command::functions::ListenerExecution::PublishResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::KillAll(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::KillAllRequestSchema(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::KillAllResponseSchema(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::laboratories::ListenerExecution::Create(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::laboratories::ListenerExecution::CreateRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::laboratories::ListenerExecution::CreateResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::laboratories::ListenerExecution::List(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::laboratories::ListenerExecution::ListRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Laboratories(objectiveai_sdk::cli::command::laboratories::ListenerExecution::ListResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Address(objectiveai_sdk::cli::command::mcp::config::address::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Address(objectiveai_sdk::cli::command::mcp::config::address::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Address(objectiveai_sdk::cli::command::mcp::config::address::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Address(objectiveai_sdk::cli::command::mcp::config::address::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Address(objectiveai_sdk::cli::command::mcp::config::address::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Address(objectiveai_sdk::cli::command::mcp::config::address::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Get(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::GetRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::GetResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Port(objectiveai_sdk::cli::command::mcp::config::port::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Port(objectiveai_sdk::cli::command::mcp::config::port::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Port(objectiveai_sdk::cli::command::mcp::config::port::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Port(objectiveai_sdk::cli::command::mcp::config::port::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Port(objectiveai_sdk::cli::command::mcp::config::port::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Config(objectiveai_sdk::cli::command::mcp::config::ListenerExecution::Port(objectiveai_sdk::cli::command::mcp::config::port::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Kill(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::KillRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::KillResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::Spawn(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::SpawnRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Mcp(objectiveai_sdk::cli::command::mcp::ListenerExecution::SpawnResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Get(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::GetRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::GetResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Install(objectiveai_sdk::cli::command::plugins::install::ListenerExecution::Filesystem(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Install(objectiveai_sdk::cli::command::plugins::install::ListenerExecution::FilesystemRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Install(objectiveai_sdk::cli::command::plugins::install::ListenerExecution::FilesystemResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Install(objectiveai_sdk::cli::command::plugins::install::ListenerExecution::Github(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Install(objectiveai_sdk::cli::command::plugins::install::ListenerExecution::GithubRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Install(objectiveai_sdk::cli::command::plugins::install::ListenerExecution::GithubResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::List(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::ListRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::ListResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Logs(objectiveai_sdk::cli::command::plugins::logs::ListenerExecution::List(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Logs(objectiveai_sdk::cli::command::plugins::logs::ListenerExecution::ListRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Logs(objectiveai_sdk::cli::command::plugins::logs::ListenerExecution::ListResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::Run(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::RunRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Plugins(objectiveai_sdk::cli::command::plugins::ListenerExecution::RunResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Python(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::PythonRequestSchema(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::PythonResponseSchema(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::Get(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::GetRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::GetResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::List(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::ListRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::ListResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::Publish(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::PublishRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Swarms(objectiveai_sdk::cli::command::swarms::ListenerExecution::PublishResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Get(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::GetRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::GetResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Install(objectiveai_sdk::cli::command::tools::install::ListenerExecution::Filesystem(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Install(objectiveai_sdk::cli::command::tools::install::ListenerExecution::FilesystemRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Install(objectiveai_sdk::cli::command::tools::install::ListenerExecution::FilesystemResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Install(objectiveai_sdk::cli::command::tools::install::ListenerExecution::Github(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Install(objectiveai_sdk::cli::command::tools::install::ListenerExecution::GithubRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Install(objectiveai_sdk::cli::command::tools::install::ListenerExecution::GithubResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::List(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::ListRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::ListResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::Run(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::RunRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Tools(objectiveai_sdk::cli::command::tools::ListenerExecution::RunResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Update(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(execution.response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::UpdateRequestSchema(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::UpdateResponseSchema(execution) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Address(objectiveai_sdk::cli::command::viewer::config::address::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Address(objectiveai_sdk::cli::command::viewer::config::address::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Address(objectiveai_sdk::cli::command::viewer::config::address::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Address(objectiveai_sdk::cli::command::viewer::config::address::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Address(objectiveai_sdk::cli::command::viewer::config::address::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Address(objectiveai_sdk::cli::command::viewer::config::address::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Get(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::GetRequestSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::GetResponseSchema(execution))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Secret(objectiveai_sdk::cli::command::viewer::config::secret::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Secret(objectiveai_sdk::cli::command::viewer::config::secret::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Secret(objectiveai_sdk::cli::command::viewer::config::secret::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Secret(objectiveai_sdk::cli::command::viewer::config::secret::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Secret(objectiveai_sdk::cli::command::viewer::config::secret::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Secret(objectiveai_sdk::cli::command::viewer::config::secret::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Signature(objectiveai_sdk::cli::command::viewer::config::signature::ListenerExecution::Get(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Signature(objectiveai_sdk::cli::command::viewer::config::signature::ListenerExecution::GetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Signature(objectiveai_sdk::cli::command::viewer::config::signature::ListenerExecution::GetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Signature(objectiveai_sdk::cli::command::viewer::config::signature::ListenerExecution::Set(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Signature(objectiveai_sdk::cli::command::viewer::config::signature::ListenerExecution::SetRequestSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Config(objectiveai_sdk::cli::command::viewer::config::ListenerExecution::Signature(objectiveai_sdk::cli::command::viewer::config::signature::ListenerExecution::SetResponseSchema(execution)))) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::GenerateSecretSignaturePair(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::GenerateSecretSignaturePairRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::GenerateSecretSignaturePairResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Kill(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::KillRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::KillResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::Spawn(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::SpawnRequestSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            objectiveai_sdk::cli::command::ListenerExecution::Viewer(objectiveai_sdk::cli::command::viewer::ListenerExecution::SpawnResponseSchema(execution)) => {
                let request = serde_json::to_value(&execution.request)
                    .unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments: execution.agent_arguments,
                    response: SerializedListenerResponse::Unary(Box::pin(async move {
                        execution.response.await.map(|value| {
                            serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
                        })
                    })),
                }
            }
            ListenerExecution::Transformed {
                request,
                agent_arguments,
                response,
            } => {
                let request =
                    serde_json::to_value(&request).unwrap_or(serde_json::Value::Null);
                SerializedListenerExecution {
                    request,
                    agent_arguments,
                    response: SerializedListenerResponse::Stream(Box::pin(
                        futures::StreamExt::map(response, |item| {
                            item.map(|value| {
                                serde_json::to_value(value)
                                    .unwrap_or(serde_json::Value::Null)
                            })
                        }),
                    )),
                }
            }
    }
}
