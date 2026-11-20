# Intégration de PROCUREMENT V0.3 avec CPL B6

Aucun changement n'est requis dans CPL ni dans PI. Le runner produit un fichier
JSON ; l'enregistrement dans CPL reste à la charge de l'appelant, comme en V0.2.

## Chaîne

```text
appel d'offres (texte UTF-8)
        │  procurement TENDER_ID TITLE SOURCE_FILE
        ▼
charge utile JSON  cm.procurement.requirements.v1 (profil 1.1.0)
        │  admit_execution(runner_type="PROCUREMENT", runner_version="0.3.0")
        │  register_artifact(schema_name="cm.procurement.requirements.v1",
        │                    schema_version="1.0.0", payload=...)
        ▼
RunnerArtifact VALIDATED (CPL B6)
```

## Paramètres d'enregistrement

| Paramètre | Valeur |
|---|---|
| `runner_type` | `PROCUREMENT` |
| `runner_version` | `0.3.0` |
| `artifact_type` | `TENDER_REQUIREMENTS` |
| `schema_name` | `cm.procurement.requirements.v1` |
| `schema_version` | `1.0.0` (voir ci-dessous) |
| `semantic_function` / `lifecycle_role` / `presentation_role` | inchangés : `EXECUTION_OUTPUT_CARRIER` / `FINAL` / `INTERNAL` |

### Registre B6 et `schema_version`

B6 ne valide un artifact que si le couple (`schema_name`, `schema_version`)
déclaré a une ligne dans `cpl.runner_artifact_schema_definitions`. Sans cette
ligne, `register_artifact` renvoie `SUCCESS` mais l'artifact est `REJECTED` :
il faut toujours contrôler `artifact_status`, pas le résultat de l'appel.

`tests/register_contract_b6.py` enregistre un contrat de `contracts/` dans le
registre de la base de test. Il refuse toute autre base, est idempotent et
n'écrase jamais un contrat existant de contenu différent.

```bash
python3 tests/register_contract_b6.py contracts/procurement_requirements_v1.json
python3 tests/register_contract_b6.py contracts/procurement_requirements_v1_1.json
```

- **`1.0.0`** : la charge utile 1.1.0 a exactement les cinq champs racine du
  contrat 1.0.0, avec les mêmes types. Elle est validée sous cette version dès
  que le contrat 1.0.0 est au registre, ce que V0.2 exigeait déjà. C'est la
  valeur par défaut du test d'intégration.
- **`1.1.0`** : demande l'enregistrement du contrat 1.1.0 (commande ci-dessus),
  puis `CPL_SCHEMA_VERSION=1.1.0`.

Dans les deux cas, aucun changement de code n'est nécessaire dans CPL ou PI :
l'enregistrement d'un contrat est une ligne de données dans le registre.

## Test d'intégration

`tests/test_procurement_v03_integration.py` reprend le test V0.2 : mêmes appels
CPL, même garde sur la base, même rollback final.

```bash
cd runners/procurement && cargo build --release && cd ../..

export DATABASE_URL=postgresql://…/capability_market_v1_test
export PROCUREMENT_BIN=$PWD/runners/procurement/target/release/procurement   # optionnel
python3 tests/test_procurement_v03_integration.py                          # sous 1.0.0
CPL_SCHEMA_VERSION=1.1.0 python3 tests/test_procurement_v03_integration.py # sous 1.1.0
```

Avec SQLAlchemy 2.1 ou plus récent, une URL `postgresql://` sélectionne le
pilote `psycopg` (version 3), alors que CPL déclare `psycopg2-binary`. Si
`psycopg` n'est pas installé, écrire l'URL `postgresql+psycopg2://…`.

Le test :

1. refuse de démarrer si `DATABASE_URL` ne cible pas `capability_market_v1_test`,
   puis vérifie `current_database()` dans la session ;
2. si `PROCUREMENT_BIN` est défini, exécute le runner et exige une sortie
   identique à `procurement_output_v03.json` ;
3. vérifie localement la charge utile (7 exigences, catégories, preuves exactes) ;
4. crée contact, actif, dossier et exécution, enregistre l'artefact ;
5. exige `artifact_status == "VALIDATED"`, `structurally_valid is True`, et une
   charge utile enregistrée identique à celle du runner ;
6. annule tout par rollback.

Sortie attendue :

```text
DATABASE: capability_market_v1_test
CONTACT: SUCCESS
ASSET: SUCCESS
CASE: SUCCESS
EXECUTION: SUCCESS
ARTIFACT: SUCCESS
ARTIFACT STATUS: VALIDATED
STRUCTURALLY VALID: True
SCHEMA VERSION: 1.0.0
RUST PROCUREMENT V0.3 -> CPL B6: PASS
ROLLBACK: donnees de test non conservees
```

## Vérification sans CPL

`tests/test_procurement_v03_contract.py` ne demande ni CPL ni base. Il valide les
deux fichiers de contrat avec `install_contracts.py`, applique aux sorties de
référence la règle structurelle déclarée par ces contrats, et les valide contre
le JSON Schema si `jsonschema` est installé.

```bash
python3 -m unittest tests/test_procurement_v03_contract.py -v
```

## Règle de validation de B6

Vérifiée dans CPL (`app/cpl/runners/artifacts.py`, fonction
`_validate_structure`, commit `3ccdf63`) et par l'exécution du test
d'intégration sous les deux versions :

- chaque champ de `required_fields` doit être présent à la racine ;
- chaque champ racine déclaré dans `field_types` doit avoir le type indiqué ;
- le contenu des tableaux n'est pas contrôlé, et un champ racine non déclaré
  est toléré.

L'extension 1.1.0, logée dans les éléments des tableaux, est donc compatible
par construction.

## Consommateurs existants

- `procurement_output.json` et `procurement_output_v02.json` ne sont pas modifiés ;
  les tests d'intégration V0.1 et V0.2 restent valables tels quels.
- Pour obtenir la sortie V0.2 à l'identique : `--contract-version 1.0.0`.
- Pour ne garder que les catégories de V0.2 dans une sortie 1.1.0 : filtrer
  `requirements` sur `binding == true`.
