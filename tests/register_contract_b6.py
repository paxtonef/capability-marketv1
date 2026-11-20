"""Enregistre un contrat dans le registre B6 de la base de test.

Ecrit une ligne dans cpl.runner_artifact_schema_definitions a partir d'un
fichier de contracts/. Sans cette ligne, CPL B6 classe tout artifact declare
sous ce (schema_name, schema_version) en REJECTED.

    python3 tests/register_contract_b6.py contracts/procurement_requirements_v1_1.json

Garde-fous :
  - refuse toute base autre que capability_market_v1_test (URL puis
    current_database()) ;
  - valide le fichier avec install_contracts.py avant toute ecriture ;
  - idempotent : un contrat deja enregistre a l'identique n'est pas reecrit ;
  - refuse d'ecraser un contrat deja enregistre avec un contenu different.

N'utilise que le modele de CPL ; ne modifie pas le code de CPL.
Contrairement aux tests d'integration, cette ecriture est validee (commit) :
le registre est de la configuration, pas une donnee de test.
"""
import os
import sys
from pathlib import Path

from sqlalchemy import text

EXPECTED_DB = "capability_market_v1_test"
url = os.environ.get("DATABASE_URL", "")

if not url or url.rsplit("/", 1)[-1].split("?", 1)[0] != EXPECTED_DB:
    sys.exit("REFUS: DATABASE_URL doit cibler capability_market_v1_test")

if len(sys.argv) != 2:
    sys.exit("Usage: python3 tests/register_contract_b6.py CONTRACT.json")

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(Path.home() / "cpl"))

from install_contracts import validate_contract  # noqa: E402

from app.db.engine import session_scope  # noqa: E402
from app.cpl.models.runner_artifact_schema_definition import (  # noqa: E402
    RunnerArtifactSchemaDefinition,
)

try:
    contract = validate_contract(sys.argv[1])
except (OSError, ValueError, TypeError) as exc:
    sys.exit(f"INVALID: {exc}")

key = (contract["schema_name"], contract["schema_version"])

with session_scope() as session:
    try:
        actual_db = session.execute(text("SELECT current_database()")).scalar_one()
        if actual_db != EXPECTED_DB:
            raise RuntimeError(f"Base interdite: {actual_db}")
        print(f"DATABASE: {actual_db}")

        existing = session.get(RunnerArtifactSchemaDefinition, key)
        if existing is None:
            session.add(
                RunnerArtifactSchemaDefinition(
                    schema_name=key[0],
                    schema_version=key[1],
                    required_fields=contract["required_fields"],
                    field_types=contract["field_types"],
                )
            )
            session.commit()
            print(f"REGISTERED: {key[0]} @ {key[1]}")
        elif (
            existing.required_fields == contract["required_fields"]
            and existing.field_types == contract["field_types"]
        ):
            print(f"ALREADY REGISTERED: {key[0]} @ {key[1]}")
        else:
            raise RuntimeError(
                f"{key[0]} @ {key[1]} est deja enregistre avec un contenu different"
            )
    except Exception:
        session.rollback()
        raise
