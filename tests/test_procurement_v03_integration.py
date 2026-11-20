"""Test local CPL B3-B6 pour Capability Market V1 - PROCUREMENT V0.3.

Enregistre dans CPL B6 la charge utile produite par le runner V0.3
(profil 1.1.0) sur la base isolee capability_market_v1_test.

Toutes les creations sont annulees par rollback.
Aucune donnee fictive n'est conservee.

Variables d'environnement :
  DATABASE_URL          obligatoire, doit cibler capability_market_v1_test
  PROCUREMENT_BIN       optionnel : binaire du runner ; s'il est defini, la
                        charge utile est produite a la volee et comparee a la
                        sortie de reference procurement_output_v03.json
  CPL_SCHEMA_VERSION    optionnel, defaut 1.0.0. La charge utile 1.1.0 est
                        enregistrable sous 1.0.0 sans changement cote CPL ;
                        utiliser 1.1.0 une fois ce contrat installe dans CPL.
"""
import json
import os
import subprocess
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

REPO_ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = os.environ.get("CPL_SCHEMA_VERSION", "1.0.0")
RUNNER_VERSION = "0.3.0"

output_file = REPO_ROOT / "procurement_output_v03.json"
reference_text = output_file.read_text(encoding="utf-8")

runner_bin = os.environ.get("PROCUREMENT_BIN")
if runner_bin:
    produced = subprocess.run(
        [
            runner_bin,
            "CM-COMPLEX-001",
            "Services linguistiques multilingues",
            str(REPO_ROOT / "tests/fixtures/procurement/tender_complex_v1.txt"),
        ],
        check=True,
        capture_output=True,
    ).stdout.decode("utf-8")
    assert produced == reference_text, "le runner ne reproduit pas la sortie de reference"
    print("RUNNER OUTPUT: identique a la reference")

procurement_payload = json.loads(reference_text)

assert procurement_payload["tender_id"] == "CM-COMPLEX-001"
assert sorted(procurement_payload) == [
    "evidence", "requirements", "tender_id", "title", "uncertainties",
]
assert len(procurement_payload["requirements"]) == 7
assert len(procurement_payload["evidence"]) == 7

categories = [item["category"] for item in procurement_payload["requirements"]]
assert categories.count("OBLIGATION") == 3
assert categories.count("PROHIBITION") == 1
assert categories.count("CONDITIONAL_OBLIGATION") == 1
assert categories.count("OPTIONAL") == 1
assert categories.count("EXEMPTION") == 1
assert len(procurement_payload["uncertainties"]) == 1

source_bytes = (
    REPO_ROOT / "tests/fixtures/procurement/tender_complex_v1.txt"
).read_bytes()
for item in procurement_payload["evidence"]:
    excerpt = source_bytes[item["byte_start"]:item["byte_end"]].decode("utf-8")
    assert excerpt == item["excerpt"], item["id"]

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
                runner_version=RUNNER_VERSION,
                execution_purpose="CPL B6 integration test - PROCUREMENT V0.3",
                initiated_by_contact_id=contact_id,
                idempotency_key="cm-v1-test-procurement-v03-execution",
                authority=authority,
            ),
        )

        result = register_artifact(
            session,
            execution_id=execution_id,
            artifact_type="TENDER_REQUIREMENTS",
            schema_name="cm.procurement.requirements.v1",
            schema_version=SCHEMA_VERSION,
            payload=procurement_payload,
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

        assert artifact.payload == procurement_payload, (
            "la charge utile enregistree differe de la sortie du runner"
        )

        print(f"SCHEMA VERSION: {SCHEMA_VERSION}")
        print("RUST PROCUREMENT V0.3 -> CPL B6: PASS")
    finally:
        session.rollback()
        print("ROLLBACK: donnees de test non conservees")
