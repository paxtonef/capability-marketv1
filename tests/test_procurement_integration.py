"""Test local CPL B3-B6 pour Capability Market V1.

Toutes les creations sont annulees par rollback.
Aucune donnee fictive n'est conservee.
"""
import os
import sys
from pathlib import Path

from sqlalchemy import text

EXPECTED_DB = "capability_market_v1_test"
url = os.environ.get("DATABASE_URL", "")

if not url or url.rsplit("/", 1)[-1].split("?", 1)[0] != EXPECTED_DB:
    sys.exit("REFUS: DATABASE_URL doit cibler capability_market_v1_test")

sys.path.insert(0, str(Path.home() / "cpl"))

from app.db.engine import session_scope
from app.cpl.identity.authority import Authority, AuthorityContext
from app.cpl.assets.authority import AssetAuthority
from app.cpl.cases.authority import CaseAuthority
from app.cpl.runners.authority import RunnerAuthority
from app.cpl.identity.contacts import create_contact
from app.cpl.assets.creation import create_asset
from app.cpl.cases.lifecycle import create_case
from app.cpl.runners.execution import admit_execution
from app.cpl.runners.artifacts import register_artifact
from app.cpl.models.runner_artifact import RunnerArtifact

permissions = frozenset({
    Authority.CREATE_CONTACT,
    AssetAuthority.CREATE_ASSET,
    CaseAuthority.CREATE_CASE,
    RunnerAuthority.ADMIT_EXECUTION,
    RunnerAuthority.REGISTER_ARTIFACT,
})

authority = AuthorityContext(
    granted=permissions,
    actor_reference="capability-market-v1-local-test",
)


def require_success(label, result):
    print(f"{label}: {result.outcome}")
    if result.outcome != "SUCCESS" or result.object_id is None:
        raise AssertionError(
            f"{label}: {result.outcome} / {result.detail}"
        )
    return result.object_id


with session_scope() as session:
    try:
        actual_db = session.execute(
            text("SELECT current_database()")
        ).scalar_one()

        if actual_db != EXPECTED_DB:
            raise RuntimeError(f"Base interdite: {actual_db}")

        print(f"DATABASE: {actual_db}")

        contact_id = require_success(
            "CONTACT",
            create_contact(
                session,
                contact_type="ORGANIZATION",
                display_name="CM V1 Test Organization",
                authority=authority,
                idempotency_key="cm-v1-test-contact",
            ),
        )

        asset_id = require_success(
            "ASSET",
            create_asset(
                session,
                asset_domain="CAPABILITY_MARKET",
                asset_type="TENDER",
                display_name="CM V1 Test Tender",
                authority=authority,
                idempotency_key="cm-v1-test-asset",
            ),
        )

        case_id = require_success(
            "CASE",
            create_case(
                session,
                primary_contact_id=contact_id,
                asset_id=asset_id,
                domain="CAPABILITY_MARKET",
                case_type="PROCUREMENT",
                title="CM V1 Procurement Integration Test",
                authority=authority,
                idempotency_key="cm-v1-test-case",
            ),
        )

        execution_id = require_success(
            "EXECUTION",
            admit_execution(
                session,
                case_id=case_id,
                asset_id=asset_id,
                runner_type="PROCUREMENT",
                runner_version="0.1.0",
                execution_purpose="CPL B6 integration test",
                initiated_by_contact_id=contact_id,
                idempotency_key="cm-v1-test-procurement-execution",
                authority=authority,
            ),
        )

        result = register_artifact(
            session,
            execution_id=execution_id,
            artifact_type="TENDER_REQUIREMENTS",
            schema_name="cm.procurement.requirements.v1",
            schema_version="1.0.0",
            payload={
                "tender_id": "CM-V1-TEST-001",
                "title": "Test procurement requirements",
                "requirements": [],
                "evidence": [],
                "uncertainties": [],
            },
            semantic_function="EXECUTION_OUTPUT_CARRIER",
            lifecycle_role="FINAL",
            presentation_role="INTERNAL",
            authority=authority,
        )

        artifact_id = require_success("ARTIFACT", result)
        artifact = session.get(RunnerArtifact, artifact_id)

        if artifact is None:
            raise AssertionError("Artifact absent de la session")

        print(f"ARTIFACT STATUS: {artifact.artifact_status}")
        print(
            "STRUCTURALLY VALID:",
            result.payload.get("structurally_valid"),
        )

        assert artifact.artifact_status == "VALIDATED"
        assert result.payload.get("structurally_valid") is True
        assert artifact.content_hash

        print("CPL B3-B6 INTEGRATION: PASS")
    finally:
        session.rollback()
        print("ROLLBACK: donnees de test non conservees")
