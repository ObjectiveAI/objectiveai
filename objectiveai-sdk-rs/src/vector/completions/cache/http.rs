use crate::{HttpClient, HttpError};

pub async fn get_completion_votes(
    client: &HttpClient,
    request: &super::request::GetCompletionVotesRequest,
) -> Result<super::response::CompletionVotes, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "vector/completions/votes",
            Some(request),
        )
        .await
}

pub async fn get_cache_vote(
    client: &HttpClient,
    request: &super::request::CacheVoteRequest<'_>,
) -> Result<super::response::CacheVote, HttpError> {
    client
        .send_unary(
            reqwest::Method::POST,
            "vector/completions/cache",
            Some(request),
        )
        .await
}
