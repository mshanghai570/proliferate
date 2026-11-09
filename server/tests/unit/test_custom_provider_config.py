"""Custom endpoint validation and delivery through typed auth sources."""

from __future__ import annotations

import json
from unittest.mock import AsyncMock
from uuid import UUID

import pytest

from proliferate.server.agent_auth import service, state_render
from proliferate.server.api_errors import CloudApiError


def config(**overrides: str) -> dict[str, str]:
    return {
        "name": "Local models",
        "endpoint": "http://127.0.0.1:1234/v1",
        "models": "org/model-a\nmodel-b",
        **overrides,
    }


@pytest.mark.asyncio
async def test_custom_provider_allows_keyless_endpoint_and_normalizes_models(monkeypatch):
    store = AsyncMock()
    monkeypatch.setattr(service.agent_gateway_store, "create_agent_provider_config", store)
    await service.create_provider_config(
        object(),
        user_id=UUID(int=1),
        title="Local models",
        kind="openai_compatible",
        value=config(models=" org/model-a \n\nmodel-b\norg/model-a "),
    )
    assert store.await_args.kwargs["value"] == config()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "overrides",
    [
        {"endpoint": "file:///tmp/provider"},
        {"endpoint": "https://user:secret@example.com/v1"},
        {"endpoint": "https://example.com/v1?token=secret"},
        {"endpoint": "https://example.com/{env:HOME}"},
        {"models": "{file:/tmp/private}"},
        {"models": "   "},
        {"apiFormat": "unsupported"},
        {"apiKey": "{file:/tmp/private}"},
    ],
)
async def test_custom_provider_rejects_invalid_configuration(monkeypatch, overrides):
    store = AsyncMock()
    monkeypatch.setattr(service.agent_gateway_store, "create_agent_provider_config", store)
    with pytest.raises(CloudApiError):
        await service.create_provider_config(
            object(),
            user_id=UUID(int=1),
            title="Custom",
            kind="openai_compatible",
            value=config(**overrides),
        )
    store.assert_not_awaited()


@pytest.mark.parametrize(
    "api_format,package",
    [
        ("chat", "@ai-sdk/openai-compatible"),
        ("responses", "@ai-sdk/openai"),
    ],
)
def test_custom_provider_renders_exact_endpoint_and_literal_key(api_format, package):
    env = state_render._translate_provider_config_env(
        "opencode",
        "openai_compatible",
        config(apiKey="sk-example", apiFormat=api_format),
        provider_id="custom-connection",
    )
    content = json.loads(env["OPENCODE_CONFIG_CONTENT"])
    provider = content["provider"]["custom-connection"]
    assert provider["npm"] == package
    assert provider["name"] == "Local models"
    assert provider["options"]["baseURL"] == "http://127.0.0.1:1234/v1"
    assert provider["options"]["apiKey"] == "{env:PROLIFERATE_CUSTOM_CONNECTION_KEY}"
    assert env["PROLIFERATE_CUSTOM_CONNECTION_KEY"] == "sk-example"
    assert set(provider["models"]) == {"org/model-a", "model-b"}
    assert "sk-example" not in env["OPENCODE_CONFIG_CONTENT"]


@pytest.mark.parametrize("api_format", ["chat", "responses"])
def test_custom_provider_keyless_config_cannot_inherit_an_ambient_key(api_format):
    env = state_render._translate_provider_config_env(
        "opencode",
        "openai_compatible",
        config(apiFormat=api_format),
        provider_id="custom-keyless",
    )
    provider = json.loads(env["OPENCODE_CONFIG_CONTENT"])["provider"]["custom-keyless"]
    assert provider["options"]["apiKey"] == ""
    assert len(env) == 1


def test_custom_provider_has_no_recipe_for_other_harnesses():
    assert (
        state_render._translate_provider_config_env(
            "claude",
            "openai_compatible",
            config(),
            provider_id="custom-connection",
        )
        is None
    )
