"""ObjectiveAI API client."""

from __future__ import annotations

import os
from typing import Any, TypeVar

import httpx

from objectiveai_sdk.error import ObjectiveAIFetchError
from objectiveai_sdk.stream import Stream

T = TypeVar("T")

DEFAULT_ADDRESS = "https://api.objectiveai.dev"


class ObjectiveAI:
    """Client for the ObjectiveAI API.

    Args:
        address: Base URL for the API.
            Falls back to ``OBJECTIVEAI_ADDRESS`` env var,
            then ``https://api.objectiveai.dev``.
        authorization: API key for authentication.
            Falls back to ``OBJECTIVEAI_AUTHORIZATION`` env var.
        user_agent: ``User-Agent`` header.
            Falls back to ``USER_AGENT`` env var.
        http_referer: ``HTTP-Referer`` header.
            Falls back to ``HTTP_REFERER`` env var.
        x_title: ``X-Title`` header.
            Falls back to ``X_TITLE`` env var.
        x_github_authorization: ``X-GITHUB-AUTHORIZATION`` header
            for GitHub-hosted function/profile access.
        x_openrouter_authorization: ``X-OPENROUTER-AUTHORIZATION`` header
            for BYOK (Bring Your Own Key) support.
        x_mcp_authorization: Map from MCP server URL to authorization
            header value, sent as ``X-MCP-AUTHORIZATION``.
        x_viewer_signature: ``X-VIEWER-SIGNATURE`` header
            for viewer authentication.
        x_viewer_address: ``X-VIEWER-ADDRESS`` header
            for viewer address.
        timeout: Request timeout in seconds (default 60).

    Usage::

        from objectiveai import ObjectiveAI

        client = ObjectiveAI(authorization="apk_...")
    """

    def __init__(
        self,
        *,
        address: str | None = None,
        authorization: str | None = None,
        user_agent: str | None = None,
        http_referer: str | None = None,
        x_title: str | None = None,
        x_github_authorization: str | None = None,
        x_openrouter_authorization: str | None = None,
        x_mcp_authorization: dict[str, str] | None = None,
        x_viewer_signature: str | None = None,
        x_viewer_address: str | None = None,
        agent_id: str | None = None,
        timeout: float = 60.0,
    ) -> None:
        self.address = (
            address
            or os.environ.get("OBJECTIVEAI_ADDRESS")
            or DEFAULT_ADDRESS
        )
        self.authorization = authorization or os.environ.get("OBJECTIVEAI_AUTHORIZATION")
        self.user_agent = user_agent or os.environ.get("USER_AGENT")
        self.http_referer = http_referer or os.environ.get("HTTP_REFERER")
        self.x_title = x_title or os.environ.get("X_TITLE")
        self.x_github_authorization = x_github_authorization or os.environ.get("GITHUB_AUTHORIZATION")
        self.x_openrouter_authorization = x_openrouter_authorization or os.environ.get("OPENROUTER_AUTHORIZATION")
        if x_mcp_authorization is not None:
            self.x_mcp_authorization = x_mcp_authorization
        else:
            raw = os.environ.get("MCP_AUTHORIZATION")
            if raw:
                try:
                    import json
                    parsed = json.loads(raw)
                    if isinstance(parsed, dict):
                        self.x_mcp_authorization = parsed
                    else:
                        self.x_mcp_authorization = None
                except (json.JSONDecodeError, TypeError):
                    self.x_mcp_authorization = None
            else:
                self.x_mcp_authorization = None
        self.x_viewer_signature = x_viewer_signature or os.environ.get("VIEWER_SIGNATURE")
        self.x_viewer_address = x_viewer_address or os.environ.get("VIEWER_ADDRESS")
        self.agent_id = agent_id or os.environ.get("OBJECTIVEAI_AGENT_ID")
        self.timeout = timeout

    def _build_headers(
        self,
        extra_headers: dict[str, str] | None = None,
    ) -> dict[str, str]:
        """Build headers for a request."""
        headers: dict[str, str] = {"Content-Type": "application/json"}

        if self.authorization:
            headers["Authorization"] = f"Bearer {self.authorization}"
        if self.user_agent:
            headers["User-Agent"] = self.user_agent
        if self.http_referer:
            headers["HTTP-Referer"] = self.http_referer
        if self.x_title:
            headers["X-Title"] = self.x_title
        if self.x_github_authorization:
            headers["X-GITHUB-AUTHORIZATION"] = self.x_github_authorization
        if self.x_openrouter_authorization:
            headers["X-OPENROUTER-AUTHORIZATION"] = self.x_openrouter_authorization
        if self.x_mcp_authorization:
            import json
            headers["X-MCP-AUTHORIZATION"] = json.dumps(self.x_mcp_authorization)
        if self.x_viewer_signature:
            headers["X-VIEWER-SIGNATURE"] = self.x_viewer_signature
        if self.x_viewer_address:
            headers["X-VIEWER-ADDRESS"] = self.x_viewer_address
        if self.agent_id:
            headers["X-OBJECTIVEAI-AGENT-ID"] = self.agent_id

        if extra_headers:
            headers.update(extra_headers)

        return headers

    def _build_url(self, path: str) -> str:
        """Build the full URL for a path."""
        base = self.address.rstrip("/")
        if not path.startswith("/"):
            path = f"/{path}"
        return f"{base}{path}"

    @staticmethod
    async def _handle_error_response(response: httpx.Response) -> ObjectiveAIFetchError:
        """Create an error from a failed response."""
        try:
            raw_body = response.text
        except Exception:
            raw_body = None
        return ObjectiveAIFetchError(response.status_code, raw_body)

    # ------------------------------------------------------------------
    # Unary requests
    # ------------------------------------------------------------------

    async def get_unary(
        self,
        path: str,
        *,
        headers: dict[str, str] | None = None,
    ) -> Any:
        """Perform a GET request and return the parsed JSON response."""
        async with httpx.AsyncClient(timeout=self.timeout) as http:
            response = await http.request(
                "GET",
                self._build_url(path),
                headers=self._build_headers(headers),
            )
        if not response.is_success:
            raise await self._handle_error_response(response)
        return response.json()

    async def post_unary(
        self,
        path: str,
        body: Any = None,
        *,
        headers: dict[str, str] | None = None,
    ) -> Any:
        """Perform a POST request and return the parsed JSON response."""
        async with httpx.AsyncClient(timeout=self.timeout) as http:
            response = await http.request(
                "POST",
                self._build_url(path),
                headers=self._build_headers(headers),
                content=_json_body(body),
            )
        if not response.is_success:
            raise await self._handle_error_response(response)
        return response.json()

    async def post_unary_no_response(
        self,
        path: str,
        body: Any = None,
        *,
        headers: dict[str, str] | None = None,
    ) -> None:
        """POST that returns no body. Any 2xx is success; non-2xx raises."""
        async with httpx.AsyncClient(timeout=self.timeout) as http:
            response = await http.request(
                "POST",
                self._build_url(path),
                headers=self._build_headers(headers),
                content=_json_body(body),
            )
        if not response.is_success:
            raise await self._handle_error_response(response)

    async def delete_unary(
        self,
        path: str,
        body: Any = None,
        *,
        headers: dict[str, str] | None = None,
    ) -> Any:
        """Perform a DELETE request and return the parsed JSON response."""
        async with httpx.AsyncClient(timeout=self.timeout) as http:
            response = await http.request(
                "DELETE",
                self._build_url(path),
                headers=self._build_headers(headers),
                content=_json_body(body),
            )
        if not response.is_success:
            raise await self._handle_error_response(response)
        return response.json()

    # ------------------------------------------------------------------
    # Streaming requests
    # ------------------------------------------------------------------

    async def get_streaming(
        self,
        path: str,
        *,
        headers: dict[str, str] | None = None,
    ) -> Stream[Any]:
        """Perform a GET request and return an SSE stream."""
        h = self._build_headers(headers)
        h["Accept"] = "text/event-stream"
        # The API defaults to WS for streaming endpoints; opt into SSE.
        h["X-Transport"] = "sse"

        http = httpx.AsyncClient(timeout=self.timeout)
        response = await http.send(
            http.build_request(
                "GET",
                self._build_url(path),
                headers=h,
            ),
            stream=True,
        )

        if not response.is_success:
            await response.aread()
            await http.aclose()
            raise await self._handle_error_response(response)

        return Stream(response)

    async def post_streaming(
        self,
        path: str,
        body: Any = None,
        *,
        headers: dict[str, str] | None = None,
    ) -> Stream[Any]:
        """Perform a POST request and return an SSE stream."""
        h = self._build_headers(headers)
        h["Accept"] = "text/event-stream"
        # The API defaults to WS for streaming endpoints; opt into SSE.
        h["X-Transport"] = "sse"

        http = httpx.AsyncClient(timeout=self.timeout)
        response = await http.send(
            http.build_request(
                "POST",
                self._build_url(path),
                headers=h,
                content=_json_body(body),
            ),
            stream=True,
        )

        if not response.is_success:
            await response.aread()
            await http.aclose()
            raise await self._handle_error_response(response)

        return Stream(response)

    async def delete_streaming(
        self,
        path: str,
        body: Any = None,
        *,
        headers: dict[str, str] | None = None,
    ) -> Stream[Any]:
        """Perform a DELETE request and return an SSE stream."""
        h = self._build_headers(headers)
        h["Accept"] = "text/event-stream"
        # The API defaults to WS for streaming endpoints; opt into SSE.
        h["X-Transport"] = "sse"

        http = httpx.AsyncClient(timeout=self.timeout)
        response = await http.send(
            http.build_request(
                "DELETE",
                self._build_url(path),
                headers=h,
                content=_json_body(body),
            ),
            stream=True,
        )

        if not response.is_success:
            await response.aread()
            await http.aclose()
            raise await self._handle_error_response(response)

        return Stream(response)


def _json_body(body: Any) -> bytes | None:
    """Serialize body to JSON bytes, or None if absent."""
    if body is None:
        return None
    import json
    return json.dumps(body).encode("utf-8")
