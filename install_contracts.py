"""Capability Market V1 — validation locale des contrats CPL B6."""
import json
import sys
from pathlib import Path

ALLOWED_TYPES = {"string", "number", "boolean", "object", "array"}

def validate_contract(path):
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    required_keys = {"schema_name", "schema_version", "required_fields", "field_types"}

    if not isinstance(data, dict) or not required_keys.issubset(data):
        raise ValueError("Missing required contract keys")

    if not all(isinstance(data[k], str) and data[k] for k in ("schema_name", "schema_version")):
        raise ValueError("Invalid schema identity")

    fields = data["required_fields"]
    types = data["field_types"]

    if not isinstance(fields, list) or not all(isinstance(f, str) and f for f in fields):
        raise ValueError("Invalid required_fields")
    if len(fields) != len(set(fields)):
        raise ValueError("Duplicate required field")
    if not isinstance(types, dict) or not all(
        isinstance(k, str) and k and v in ALLOWED_TYPES
        for k, v in types.items()
    ):
        raise ValueError("Invalid field_types")
    if not set(fields).issubset(types):
        raise ValueError("Missing type declarations for required fields")

    print(f"VALID: {data['schema_name']} @ {data['schema_version']}")
    return data

if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("Usage: python3 install_contracts.py CONTRACT.json")
    try:
        validate_contract(sys.argv[1])
    except (OSError, ValueError, TypeError) as exc:
        sys.exit(f"INVALID: {exc}")
