"""Verification locale du contrat cm.procurement.requirements.v1 (1.0.0 et 1.1.0).

Ne necessite ni CPL ni base de donnees. Reproduit la regle structurelle du
contrat CPL B6 telle qu'elle est decrite par les fichiers de contracts/ :
champs requis presents, types racine conformes.

    python3 -m unittest tests/test_procurement_v03_contract.py -v

Variable optionnelle : PROCUREMENT_BIN (chemin du binaire) pour verifier que
le runner reproduit les sorties de reference.
"""
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from install_contracts import validate_contract  # noqa: E402

CONTRACT_V1_0 = ROOT / "contracts/procurement_requirements_v1.json"
CONTRACT_V1_1 = ROOT / "contracts/procurement_requirements_v1_1.json"
SCHEMA_V1_1 = ROOT / "contracts/procurement_requirements_v1_1.schema.json"
FIXTURES = ROOT / "tests/fixtures/procurement"

OUTPUT_V02 = ROOT / "procurement_output_v02.json"
OUTPUT_V03 = ROOT / "procurement_output_v03.json"
EXPECTED_MODALITIES = FIXTURES / "tender_v03_modalities.expected.json"

PYTHON_TYPES = {
    "string": str,
    "number": (int, float),
    "boolean": bool,
    "object": dict,
    "array": list,
}


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def structural_errors(payload, contract):
    """Regle structurelle B6 : champs requis et types declares."""
    errors = []
    for field in contract["required_fields"]:
        if field not in payload:
            errors.append(f"champ requis absent: {field}")
    for field, kind in contract["field_types"].items():
        if field in payload and not isinstance(payload[field], PYTHON_TYPES[kind]):
            errors.append(f"type invalide pour {field}: attendu {kind}")
    return errors


class ContractFiles(unittest.TestCase):
    def test_both_contracts_are_valid_b6_contracts(self):
        self.assertEqual(validate_contract(CONTRACT_V1_0)["schema_version"], "1.0.0")
        self.assertEqual(validate_contract(CONTRACT_V1_1)["schema_version"], "1.1.0")

    def test_1_1_keeps_the_root_structure_of_1_0(self):
        old, new = load(CONTRACT_V1_0), load(CONTRACT_V1_1)
        self.assertEqual(new["schema_name"], old["schema_name"])
        self.assertEqual(new["required_fields"], old["required_fields"])
        self.assertEqual(new["field_types"], old["field_types"])

    def test_schema_root_matches_the_b6_contract(self):
        schema, contract = load(SCHEMA_V1_1), load(CONTRACT_V1_1)
        self.assertEqual(sorted(schema["required"]), sorted(contract["required_fields"]))
        for field, kind in contract["field_types"].items():
            self.assertEqual(schema["properties"][field]["type"], kind)


class Payloads(unittest.TestCase):
    def test_every_reference_output_satisfies_both_contracts(self):
        for output in (OUTPUT_V02, OUTPUT_V03, EXPECTED_MODALITIES):
            for contract in (CONTRACT_V1_0, CONTRACT_V1_1):
                with self.subTest(output=output.name, contract=contract.name):
                    self.assertEqual(structural_errors(load(output), load(contract)), [])

    def test_1_1_outputs_add_no_root_field(self):
        for output in (OUTPUT_V03, EXPECTED_MODALITIES):
            self.assertEqual(sorted(load(output)), sorted(load(CONTRACT_V1_0)["required_fields"]))

    def test_1_1_outputs_keep_every_1_0_item_field(self):
        old, new = load(OUTPUT_V02), load(OUTPUT_V03)
        for key in ("requirements", "evidence", "uncertainties"):
            for old_item, new_item in zip(old[key], new[key]):
                for field, value in old_item.items():
                    self.assertEqual(new_item[field], value, f"{key}.{field}")

    def test_1_1_outputs_validate_against_the_json_schema(self):
        try:
            import jsonschema
        except ImportError:
            self.skipTest("jsonschema non installe (pip install jsonschema)")
        schema = load(SCHEMA_V1_1)
        jsonschema.Draft202012Validator.check_schema(schema)
        validator = jsonschema.Draft202012Validator(schema)
        for output in (OUTPUT_V03, EXPECTED_MODALITIES):
            with self.subTest(output=output.name):
                self.assertEqual([e.message for e in validator.iter_errors(load(output))], [])
        # Le schema est discriminant : il refuse une charge utile alteree.
        broken = load(OUTPUT_V03)
        broken["requirements"][0]["category"] = "SUGGESTION"
        self.assertTrue(list(validator.iter_errors(broken)))

    def test_evidence_excerpts_are_exact_source_bytes(self):
        cases = (
            (OUTPUT_V03, FIXTURES / "tender_complex_v1.txt"),
            (EXPECTED_MODALITIES, FIXTURES / "tender_v03_modalities.txt"),
        )
        for output, source_path in cases:
            source = source_path.read_bytes()
            payload = load(output)
            self.assertTrue(payload["evidence"])
            for item in payload["evidence"]:
                excerpt = source[item["byte_start"]:item["byte_end"]].decode("utf-8")
                self.assertEqual(excerpt, item["excerpt"])
            for requirement in payload["requirements"]:
                for condition in requirement["conditions"]:
                    clause = source[condition["byte_start"]:condition["byte_end"]]
                    self.assertEqual(clause.decode("utf-8"), condition["text"])


@unittest.skipUnless(os.environ.get("PROCUREMENT_BIN"), "PROCUREMENT_BIN non defini")
class Runner(unittest.TestCase):
    def run_runner(self, *args):
        result = subprocess.run(
            [os.environ["PROCUREMENT_BIN"], *args],
            check=True,
            capture_output=True,
        )
        return result.stdout.decode("utf-8")

    def test_runner_reproduces_reference_outputs(self):
        source = str(FIXTURES / "tender_complex_v1.txt")
        title = "Services linguistiques multilingues"
        self.assertEqual(
            self.run_runner("CM-COMPLEX-001", title, source),
            OUTPUT_V03.read_text(encoding="utf-8"),
        )
        self.assertEqual(
            self.run_runner("--contract-version", "1.0.0", "CM-COMPLEX-001", title, source),
            OUTPUT_V02.read_text(encoding="utf-8"),
        )


if __name__ == "__main__":
    unittest.main()
