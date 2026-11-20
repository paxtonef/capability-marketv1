# Intégration CPL B6 — PROCUREMENT V0.3

| | |
|---|---|
| Date | 2026-11-20 |
| Commit testé | `d8d675e71441d66fa007995f4ded6b6e11e22d2a` |
| CPL | `paxtonef/cpl` @ `3ccdf63a8e2fd2879bbb9eea71053ab11c176e7c`, non modifié |
| Base | `capability_market_v1_test`, PostgreSQL 16.15, migrations CPL 001–028 |
| Runner | `procurement 0.3.0`, binaire release |
| Charge utile | sortie du runner sur `tender_complex_v1.txt`, identique à `procurement_output_v03.json` |

**Portée.** L'instance PostgreSQL a été montée pour cette validation : base
créée vide, schéma issu des migrations de CPL. Ce n'est pas votre base de test
existante. Le code exécuté est celui de CPL, sans mock.

## Résultats

| | Contrat 1.0.0 | Contrat 1.1.0 |
|---|---|---|
| Contrat présent au registre B6 | oui (enregistré en §3.3) | oui (enregistré en §3.7) |
| `create_contact` / `create_asset` / `create_case` | SUCCESS | SUCCESS |
| `admit_execution` (`PROCUREMENT`, `0.3.0`) | SUCCESS | SUCCESS |
| `register_artifact` (`TENDER_REQUIREMENTS`) | SUCCESS | SUCCESS |
| **`artifact_status`** (relu en SQL) | **`VALIDATED`** | **`VALIDATED`** |
| **`structurally_valid`** | **`True`** | **`True`** |
| `schema_name` / `schema_version` enregistrés | `cm.procurement.requirements.v1` / `1.0.0` | `cm.procurement.requirements.v1` / `1.1.0` |
| Intégrité du contenu (`verify_content_integrity`) | `MATCH` | `MATCH` |
| Charge utile relue = charge utile du runner | identique | identique |
| Éléments de `requirements` / dont `binding` | 7 / 5 | 7 / 5 |
| Rollback | effectué | effectué |
| Seconde exécution | PASS | PASS |

Le statut a été contrôlé de deux façons : par le test du dépôt
(`tests/test_procurement_v03_integration.py`, via `session.get`) et par une
relecture SQL directe de `cpl.runner_artifacts` jointe à `cpl.runner_executions`
(`verify_artifact_status.py`, outil de validation hors dépôt).

## Le succès de l'appel ne prouve rien : témoins négatifs

| Situation | Appel `register_artifact` | `artifact_status` | Test |
|---|---|---|---|
| Registre vide, version 1.0.0 demandée | SUCCESS | `REJECTED` | échec, comme attendu |
| 1.0.0 seul au registre, version 1.1.0 demandée | SUCCESS | `REJECTED` | échec, comme attendu |
| Contrat demandé présent au registre | SUCCESS | `VALIDATED` | PASS |

Le test distingue donc bien un artifact validé d'un appel simplement accepté.

## Registre B6 après validation

```text
schema_name                      schema_version  required_fields
cm.procurement.requirements.v1   1.0.0           tender_id, title, requirements, evidence, uncertainties
cm.procurement.requirements.v1   1.1.0           tender_id, title, requirements, evidence, uncertainties
```

`field_types` est identique pour les deux : `tender_id` et `title` en `string`,
les trois autres en `array`.

Le contrat 1.1.0 a été enregistré par `tests/register_contract_b6.py`. Le script
refuse toute base autre que `capability_market_v1_test`, valide le fichier avant
d'écrire, ne réécrit pas un contrat déjà présent à l'identique et refuse d'en
écraser un différent. C'est la seule écriture persistante de toute la
validation : une ligne de configuration par contrat, pas une donnée de test.

## Pourquoi les deux versions passent sans changement dans CPL

`_validate_structure` (`app/cpl/runners/artifacts.py`) applique trois règles :
le couple (`schema_name`, `schema_version`) doit être au registre ; chaque champ
de `required_fields` doit être présent à la racine ; chaque champ racine déclaré
dans `field_types` doit avoir le bon type. Le contenu des tableaux n'est pas
contrôlé. L'extension 1.1.0 tient entièrement dans les éléments des tableaux :
elle est compatible par construction, ce que l'exécution confirme.

## Non-régression V0.1 et V0.2 sur CPL

Les trois tests d'intégration antérieurs, inchangés depuis `db30fe2` :

| Test | `artifact_status` | Résultat |
|---|---|---|
| `test_procurement_integration.py` | `VALIDATED` | `CPL B3-B6 INTEGRATION: PASS` |
| `test_procurement_rust_integration.py` | `VALIDATED` | `RUST PROCUREMENT -> CPL B6: PASS` |
| `test_procurement_v02_integration.py` | `VALIDATED` | `RUST PROCUREMENT V0.2 -> CPL B6: PASS` |

## Rollback

Décompte des lignes des 31 tables du schéma `cpl`, avant puis après l'ensemble
des tests (11 exécutions d'intégration, dont 2 témoins négatifs) :

| Table | Avant | Après |
|---|---|---|
| `contacts`, `assets`, `cases` | 0 | 0 |
| `runner_executions`, `runner_artifacts` | 0 | 0 |
| `runner_governance_decisions` | 0 | 0 |
| les 24 autres tables de données | 0 | 0 |
| `runner_artifact_schema_definitions` (registre) | 0 | 2 |

Aucune donnée de test ne subsiste. La seule différence est le registre, voulue.

## Garde-fous sur la base

`tests/test_procurement_v03_integration.py` et `tests/register_contract_b6.py`
ont été lancés avec une URL visant `cpl_db`, `pi01_test`, `test` et `postgres` :
huit refus sur huit (`REFUS: DATABASE_URL doit cibler capability_market_v1_test`),
avant l'import de CPL et donc avant toute connexion. Ces bases n'existent
d'ailleurs pas sur l'instance de validation.

## Reproduire sur votre base de test

```bash
cd capability-marketv1 && git checkout feature/procurement-v0.3
(cd runners/procurement && cargo build --release)

export DATABASE_URL=postgresql+psycopg2://…/capability_market_v1_test
export PROCUREMENT_BIN=$PWD/runners/procurement/target/release/procurement

python3 tests/register_contract_b6.py contracts/procurement_requirements_v1.json
python3 tests/register_contract_b6.py contracts/procurement_requirements_v1_1.json

python3 tests/test_procurement_v03_integration.py
CPL_SCHEMA_VERSION=1.1.0 python3 tests/test_procurement_v03_integration.py
```

Sortie attendue de chaque test : `ARTIFACT STATUS: VALIDATED`,
`STRUCTURALLY VALID: True`, `RUST PROCUREMENT V0.3 -> CPL B6: PASS`,
`ROLLBACK: donnees de test non conservees`.
