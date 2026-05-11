"""Tests for the runner's MCP config.toml emitter.

Run with the runner's venv active:

    venv/bin/python -m unittest tests.test_config_toml -v
    # or on Windows:
    venv\\Scripts\\python -m unittest tests.test_config_toml -v
"""

from __future__ import annotations

import sys
import tomllib
import unittest
from pathlib import Path

# Make the runner package importable when running this test from the
# `objectiveai-codex-sdk-runner` directory.
_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parent))

from main import _build_config_toml  # noqa: E402


class TestBuildConfigToml(unittest.TestCase):
    def test_simple_url_only(self) -> None:
        toml_text = _build_config_toml({
            "fs": {"url": "https://proxy.example.com/mcp"},
        })
        parsed = tomllib.loads(toml_text)
        self.assertEqual(
            parsed,
            {
                "mcp_servers": {
                    "fs": {
                        "url": "https://proxy.example.com/mcp",
                        "required": True,
                    }
                }
            },
        )

    def test_url_with_headers(self) -> None:
        toml_text = _build_config_toml({
            "fs": {
                "url": "https://proxy.example.com/mcp",
                "http_headers": {
                    "Mcp-Session-Id": "abc-123",
                    "Authorization": "Bearer token-xyz",
                    "X-Custom": "value",
                },
            },
        })
        parsed = tomllib.loads(toml_text)
        self.assertEqual(
            parsed,
            {
                "mcp_servers": {
                    "fs": {
                        "url": "https://proxy.example.com/mcp",
                        "required": True,
                        "http_headers": {
                            "Mcp-Session-Id": "abc-123",
                            "Authorization": "Bearer token-xyz",
                            "X-Custom": "value",
                        },
                    }
                }
            },
        )

    def test_quoted_server_name(self) -> None:
        # Server names from objectiveai may be UUID-like with dashes.
        name = "550e8400-e29b-41d4-a716-446655440000"
        toml_text = _build_config_toml({
            name: {
                "url": "https://proxy.example.com/mcp",
                "http_headers": {"Mcp-Session-Id": "session"},
            },
        })
        parsed = tomllib.loads(toml_text)
        self.assertIn(name, parsed["mcp_servers"])
        self.assertEqual(
            parsed["mcp_servers"][name]["http_headers"]["Mcp-Session-Id"],
            "session",
        )

    def test_multiple_servers(self) -> None:
        toml_text = _build_config_toml({
            "a": {"url": "https://a.example/mcp"},
            "b": {"url": "https://b.example/mcp", "http_headers": {"X": "y"}},
        })
        parsed = tomllib.loads(toml_text)
        self.assertEqual(set(parsed["mcp_servers"].keys()), {"a", "b"})
        self.assertEqual(parsed["mcp_servers"]["a"]["url"], "https://a.example/mcp")
        self.assertEqual(parsed["mcp_servers"]["b"]["http_headers"], {"X": "y"})

    def test_empty_map_returns_empty_string(self) -> None:
        self.assertEqual(_build_config_toml({}), "")

    def test_string_values_with_special_chars_are_escaped(self) -> None:
        toml_text = _build_config_toml({
            "fs": {
                "url": 'https://proxy.example.com/path with "quotes"\\and\\slashes',
                "http_headers": {"X-Newline": "line1\nline2\ttab"},
            },
        })
        parsed = tomllib.loads(toml_text)
        self.assertEqual(
            parsed["mcp_servers"]["fs"]["url"],
            'https://proxy.example.com/path with "quotes"\\and\\slashes',
        )
        self.assertEqual(
            parsed["mcp_servers"]["fs"]["http_headers"]["X-Newline"],
            "line1\nline2\ttab",
        )

    def test_rejects_non_dict_top_level(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml("not a dict")  # type: ignore[arg-type]

    def test_rejects_missing_url(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({"fs": {}})

    def test_rejects_empty_url(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({"fs": {"url": ""}})

    def test_rejects_non_string_url(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({"fs": {"url": 42}})

    def test_rejects_non_dict_server_value(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({"fs": "https://x"})

    def test_rejects_non_dict_http_headers(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({
                "fs": {"url": "https://x", "http_headers": ["not", "a", "dict"]},
            })

    def test_rejects_non_string_header_value(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({
                "fs": {"url": "https://x", "http_headers": {"X": 42}},
            })

    def test_rejects_empty_server_name(self) -> None:
        with self.assertRaises(ValueError):
            _build_config_toml({"": {"url": "https://x"}})


if __name__ == "__main__":
    unittest.main()
