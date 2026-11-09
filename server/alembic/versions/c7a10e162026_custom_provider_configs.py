"""Allow encrypted OpenAI-compatible provider configurations.

Revision ID: c7a10e162026
Revises: d9e4b7a2c6f1
"""

from alembic import op

revision = "c7a10e162026"
down_revision = "d9e4b7a2c6f1"
branch_labels = None
depends_on = None


def upgrade() -> None:
    op.drop_constraint("ck_agent_api_key_kind", "agent_api_key", type_="check")
    op.create_check_constraint(
        "ck_agent_api_key_kind",
        "agent_api_key",
        "kind IN ('api_key', 'aws_bedrock', 'azure_openai', "
        "'openai_compatible', 'anthropic_subscription')",
    )


def downgrade() -> None:
    # Refuse a downgrade that would discard configured providers.
    op.drop_constraint("ck_agent_api_key_kind", "agent_api_key", type_="check")
    op.create_check_constraint(
        "ck_agent_api_key_kind",
        "agent_api_key",
        "kind IN ('api_key', 'aws_bedrock', 'azure_openai', 'anthropic_subscription')",
    )
