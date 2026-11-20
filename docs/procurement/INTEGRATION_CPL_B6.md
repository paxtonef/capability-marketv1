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

### Quelle `schema_version` déclarer

- **`1.0.0`, sans rien installer.** La charge utile 1.1.0 a exactement les cinq
  champs racine du contrat 1.0.0, avec les mêmes types. C'est la valeur par
  défaut du test d'intégration.
- **`1.1.0`, après installation du contrat dans CPL.** Le fichier
  `contracts/procurement_requirements_v1_1.json` est au format des contrats B6
  et passe `install_contracts.py`. Son installation dans le registre de CPL est
  une opération côté CPL, hors périmètre de ce dépôt. Le test se lance alors avec
  `CPL_SCHEMA_VERSION=1.1.0`.

## Test d'intégration

`tests/test_procurement_v03_integration.py` reprend le test V0.2 : mêmes appels
CPL, même garde sur la base, même rollback final.

```bash
cd runners/procurement && cargo build --release && cd ../..

export DATABASE_URL=postgresql://…/capability_market_v1_test
export PROCUREMENT_BIN=$PWD/runners/procurement/target/release/procurement   # optionnel
python3 tests/test_procurement_v03_integration.py
```

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

## Hypothèse à confirmer côté CPL

La compatibilité repose sur le fait que B6 valide les champs racine déclarés par
le contrat (`required_fields`, `field_types`) et ne contraint pas le contenu des
tableaux. Cette hypothèse découle du format du contrat et du précédent V0.2
(champ `category` ajouté aux éléments sous la version 1.0.0). Le code de CPL n'est
pas dans ce dépôt : seule l'exécution du test d'intégration la confirme.

## Consommateurs existants

- `procurement_output.json` et `procurement_output_v02.json` ne sont pas modifiés ;
  les tests d'intégration V0.1 et V0.2 restent valables tels quels.
- Pour obtenir la sortie V0.2 à l'identique : `--contract-version 1.0.0`.
- Pour ne garder que les catégories de V0.2 dans une sortie 1.1.0 : filtrer
  `requirements` sur `binding == true`.
